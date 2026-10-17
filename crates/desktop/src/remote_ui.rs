// SPDX-License-Identifier: GPL-3.0-only
use super::read_job::ReadJob;
use super::*;
use mucrab_core::{FileEntry, sftp::*};
use std::{collections::HashMap, sync::Arc};
// Secret editing uses a private String; it is moved to a zeroizing owner on submit
// and cleared on close. GUI/library temporaries cannot be guaranteed erased.
struct Inputs {
    tab: TabId,
    request: RemoteNavigation,
    profile: Profile,
    secret: String,
    remember: bool,
    use_saved: bool,
}
impl Drop for Inputs {
    fn drop(&mut self) {
        clear_secret(&mut self.secret);
    }
}
enum ConnectionResult {
    Key(Handshake, Box<Inputs>),
    Ready(Arc<RemoteSession>, TabId, RemoteNavigation, Vec<FileEntry>),
}
struct Listing {
    request: RemoteNavigation,
    job: ReadJob<Vec<FileEntry>>,
}
pub(super) struct RemoteUi {
    pub(super) visible: bool,
    store: Option<ConnectionStore>,
    profile: Profile,
    secret: String,
    remember: bool,
    use_saved: bool,
    connect: ReadJob<ConnectionResult>,
    connecting_profile: Option<String>,
    trust: Option<(Handshake, Inputs)>,
    sessions: HashMap<String, Arc<RemoteSession>>,
    listings: HashMap<TabId, Listing>,
    prepare: ReadJob<TransferPlan>,
    prepare_origin: Option<(PaneSide, TabId)>,
    review: Option<(TransferPlan, PaneSide, TabId)>,
    download: ReadJob<(PathBuf, bool, PaneSide, TabId)>,
    download_profile: Option<String>,
    credential: ReadJob<String>,
    pub(super) transfer_profiles: Vec<String>,
    pub(super) message: Option<String>,
    forget: Option<String>,
    target: Option<TabId>,
    resume: Option<RemoteNavigation>,
}
impl RemoteUi {
    pub(super) fn load() -> Self {
        let result = crate::appearance::settings_path()
            .ok_or("Configuration path unavailable".to_owned())
            .and_then(|p| {
                ConnectionStore::load(p.with_file_name("sftp.json")).map_err(|e| e.to_string())
            });
        let (store, message) = match result {
            Ok(s) => (Some(s), None),
            Err(e) => (None, Some(e)),
        };
        Self {
            visible: false,
            store,
            profile: Profile {
                id: "server-1".into(),
                label: "SFTP server".into(),
                host: String::new(),
                port: 22,
                user: String::new(),
                authentication: Authentication::Password,
                directory: "/".into(),
                remembered: false,
                credential_context: None,
            },
            secret: String::new(),
            remember: false,
            use_saved: false,
            connect: ReadJob::default(),
            connecting_profile: None,
            trust: None,
            sessions: HashMap::new(),
            listings: HashMap::new(),
            prepare: ReadJob::default(),
            prepare_origin: None,
            review: None,
            download: ReadJob::default(),
            download_profile: None,
            credential: ReadJob::default(),
            transfer_profiles: vec![],
            message,
            target: None,
            resume: None,
            forget: None,
        }
    }
    pub(super) fn target_active(&mut self, tab: TabId) {
        self.target = Some(tab);
        self.resume = None;
    }
    pub(super) fn preparation_busy(&self) -> bool {
        self.connect.busy() || self.download.busy() || self.prepare.busy()
    }
    pub(super) fn cancel_preparation(&mut self) {
        self.connecting_profile = None;
        self.connect.discard();
        self.download.discard();
        self.prepare.discard();
        self.trust = None;
        clear_secret(&mut self.secret);
    }
    pub(super) fn busy(&self) -> bool {
        self.connect.busy()
            || self.prepare.busy()
            || self.download.busy()
            || self.credential.busy()
            || self.listings.values().any(|v| v.job.busy())
    }
    pub(super) fn modal(&self) -> bool {
        self.trust.is_some() || self.review.is_some() || self.forget.is_some()
    }
    pub(super) fn cancel(&mut self) {
        self.connecting_profile = None;
        self.connect.discard();
        self.prepare.discard();
        self.download.discard();
        self.credential.discard();
        self.trust = None;
        for job in self.listings.values_mut() {
            job.job.discard();
        }
        for s in self.sessions.values() {
            s.disconnect();
        }
        clear_secret(&mut self.secret);
    }
    fn session(&self, id: &str) -> Option<Arc<RemoteSession>> {
        self.sessions.get(id).filter(|s| s.connected()).cloned()
    }
    fn authenticate(&mut self, handshake: Handshake, mut input: Inputs) {
        let verified = handshake.identity.clone();
        let store_path = crate::appearance::settings_path().map(|p| p.with_file_name("sftp.json"));
        let result = self.connect.start(move |c| {
            if input.use_saved&&!input.profile.saved_credential_matches(){return Err("Saved credential belongs to different connection settings or is unavailable; enter a new session secret.".into());}
            let secret = if input.use_saved {
                load_secret(&input.profile.id)
            } else {
                Ok(mucrab_core::sftp::session_secret(std::mem::take(
                    &mut input.secret,
                )))
            }
            .map_err(|e| e.to_string())?;
            let session = handshake
                .authenticate(input.profile.clone(), &verified, &secret, &c)
                .map_err(|e| e.to_string())?;
            let entries = session
                .list(&input.request.location, &c)
                .map_err(|e| e.to_string())?;
            if input.remember {
                save_secret(&input.profile.id, &secret).map_err(|e| e.to_string())?;
                input.profile.remembered = true;
                input.profile.credential_context=Some(input.profile.credential_context_key());
                let saved = (|| -> Result<(), String> {
                    let mut store =
                        ConnectionStore::load(store_path.ok_or("Configuration unavailable")?)
                            .map_err(|e| e.to_string())?;
                    store
                        .save_profile(input.profile.clone())
                        .map_err(|e| e.to_string())
                })();
                if let Err(e) = saved {
                    let _ = delete_secret(&input.profile.id);
                    return Err(e);
                }
            }
            Ok(ConnectionResult::Ready(
                session,
                input.tab,
                input.request.clone(),
                entries,
            ))
        });
        if let Err(e) = result {
            self.message = Some(e);
        }
    }
    fn endpoint(&self, pane: &BrowserPane) -> Option<Endpoint> {
        if let Some(location) = &pane.remote {
            if pane.remote_disconnected {
                return None;
            }
            Some(Endpoint::Remote {
                session: self.session(&location.profile)?,
                location: location.clone(),
            })
        } else if pane.archive.is_none() {
            Some(Endpoint::Local(pane.location.clone()?))
        } else {
            None
        }
    }
}
impl MuCrabApp {
    pub(super) fn can_remote_copy(&self) -> bool {
        let i = usize::from(self.active == PaneSide::Right);
        let (a, b) = (&self.panes[i], &self.panes[1 - i]);
        (a.remote.is_some() ^ b.remote.is_some())
            && self.remote.endpoint(a).is_some()
            && self.remote.endpoint(b).is_some()
            && !a
                .selection
                .snapshot(&a.entries, &a.order)
                .selected
                .is_empty()
            && !self.remote.prepare.busy()
    }
    pub(super) fn prepare_remote_copy(&mut self) {
        let i = usize::from(self.active == PaneSide::Right);
        let Some(source) = self.remote.endpoint(&self.panes[i]) else {
            return;
        };
        let Some(destination) = self.remote.endpoint(&self.panes[1 - i]) else {
            return;
        };
        let names = self.panes[i]
            .selection
            .snapshot(&self.panes[i].entries, &self.panes[i].order)
            .selected;
        self.remote.prepare_origin = Some((self.active, self.panes.stack(i).active().id));
        if let Err(e) = self.remote.prepare.start(move |c| {
            TransferPlan::capture(source, destination, names, CopyPolicy::Skip, &c)
                .map_err(|e| e.to_string())
        }) {
            self.remote.message = Some(e);
        }
    }
    pub(super) fn remote_open(&mut self, location: RemoteLocation, view: bool) {
        let Some(session) = self.remote.session(&location.profile) else {
            self.remote.message = Some("SFTP session disconnected; reconnect first".into());
            return;
        };
        if self.remote.download.busy() {
            self.remote.message = Some("Wait for the pending remote download".into());
            return;
        }
        let side = self.active;
        let i = usize::from(side == PaneSide::Right);
        let tab = self.panes.stack(i).active().id;
        self.remote.download_profile = Some(location.profile.clone());
        if let Err(e) = self.remote.download.start(move |c| {
            download_open(&session, &location, &c)
                .map(|p| (p, view, side, tab))
                .map_err(|e| e.to_string())
        }) {
            self.remote.message = Some(e);
        }
    }
    pub(super) fn dispatch_target(&mut self, target: crate::opening::Target) {
        match target {
            crate::opening::Target::Remote(location) => self.remote_open(location, false),
            target => self.opener.start(target),
        }
    }
    fn begin_connection(&mut self) {
        let i = usize::from(self.active == PaneSide::Right);
        let tab = self
            .remote
            .target
            .unwrap_or(self.panes.stack(i).active().id);
        let profile = self.remote.profile.clone();
        if let Err(e) = profile.validate() {
            self.remote.message = Some(e.to_string());
            return;
        }
        if self
            .remote
            .store
            .as_ref()
            .and_then(|s| s.profile(&profile.id))
            .is_none_or(|p| !p.same_connection(&profile))
        {
            self.remote.message=Some("Save the non-secret profile before connecting, including changed connection settings.".into());
            return;
        }
        let Some(pane) = self.panes.find_mut(tab) else {
            self.remote.message = Some("Originating tab closed".into());
            return;
        };
        let resume = self
            .remote
            .resume
            .take()
            .filter(|r| r.location.profile == profile.id);
        let location = resume
            .as_ref()
            .map(|r| r.location.clone())
            .or_else(|| {
                pane.remote_request
                    .as_ref()
                    .filter(|r| r.location.profile == profile.id)
                    .map(|r| r.location.clone())
            })
            .or_else(|| {
                pane.remote
                    .as_ref()
                    .filter(|r| r.profile == profile.id)
                    .cloned()
            })
            .unwrap_or_else(|| {
                RemoteLocation::new(&profile.id, &profile.directory).expect("validated profile")
            });
        pane.request_remote(
            location,
            resume.as_ref().is_none_or(|r| r.record),
            resume.and_then(|r| r.cursor),
        );
        let request = pane.remote_request.clone().expect("requested");
        self.remote.connecting_profile = Some(profile.id.clone());
        let input = Inputs {
            tab,
            request,
            profile,
            secret: std::mem::take(&mut self.remote.secret),
            remember: self.remote.remember,
            use_saved: self.remote.use_saved,
        };
        self.remote.message = None;
        if let Err(e) = self.remote.connect.start(move |c| {
            Handshake::connect(&input.profile, &c)
                .map(|h| ConnectionResult::Key(h, Box::new(input)))
                .map_err(|e| e.to_string())
        }) {
            self.remote.message = Some(e);
        }
    }
    fn disconnect_remote(&mut self, id: &str) {
        if self.remote.connecting_profile.as_deref() == Some(id) {
            self.remote.connect.discard();
            self.remote.trust = None;
            self.remote.connecting_profile = None;
        }
        for tab in self.panes.all_mut() {
            if tab
                .pane
                .remote_request
                .as_ref()
                .is_some_and(|r| r.location.profile == id)
            {
                tab.pane.remote_request = None;
            }
        }
        if let Some(s) = self.remote.sessions.remove(id) {
            s.disconnect();
        }
        for t in self.panes.all_mut() {
            if t.pane.remote.as_ref().is_some_and(|r| r.profile == id) {
                t.pane.remote_disconnected = true;
                t.pane.remote_request = None;
                t.pane.error = Some("SFTP disconnected; reconnect to browse or copy".into());
            }
        }
        for l in self.remote.listings.values_mut() {
            if l.request.location.profile == id {
                l.job.discard();
            }
        }
        if self.remote.transfer_profiles.iter().any(|p| p == id)
            && let Some(job) = &self.copy_job
        {
            job.control.cancel();
        }
        if self.remote.download_profile.as_deref() == Some(id) {
            self.remote.download.discard();
        }
        self.remote.message=Some("Disconnected; associated network work is cancelling. Partial transfer paths remain in Details.".into());
    }
    pub(super) fn poll_remote(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.remote.connect.poll() {
            match result {
                Ok(ConnectionResult::Key(handshake, input)) => {
                    let input = *input;
                    if !self.panes.contains(input.tab) {
                        continue_connection_drop(handshake, input);
                        return;
                    }
                    let status = self
                        .remote
                        .store
                        .as_ref()
                        .ok_or("SFTP config unavailable".to_owned())
                        .and_then(|s| {
                            s.trust_status(&handshake.identity)
                                .map_err(|e| e.to_string())
                        });
                    match status {
                        Ok(true) => self.remote.authenticate(handshake, input),
                        Ok(false) => self.remote.trust = Some((handshake, input)),
                        Err(e) => self.remote.message = Some(e),
                    }
                }
                Ok(ConnectionResult::Ready(session, tab, request, entries)) => {
                    self.remote.connecting_profile = None;
                    let trusted = self.remote.store.as_ref().is_some_and(|store| {
                        store.trust_status(&session.identity).unwrap_or(false)
                            && store
                                .profile(&session.profile.id)
                                .is_some_and(|p| p.same_connection(&session.profile))
                    });
                    if !trusted {
                        session.disconnect();
                        self.remote.message = Some(
                            "Host trust or profile changed during connection; connect again."
                                .into(),
                        );
                        if let Some(p) = self.panes.find_mut(tab) {
                            p.remote_request = None;
                        }
                        return;
                    }

                    if !self.panes.contains(tab) {
                        session.disconnect();
                        return;
                    }
                    self.remote
                        .sessions
                        .insert(session.profile.id.clone(), session);
                    if let Some(pane) = self.panes.find_mut(tab) {
                        pane.remote_loaded(&request, entries, &self.sorter);
                    }
                    if let Some(p) = crate::appearance::settings_path() {
                        self.remote.store =
                            ConnectionStore::load(p.with_file_name("sftp.json")).ok();
                    }
                    if let Some(p) = self
                        .remote
                        .store
                        .as_ref()
                        .and_then(|s| s.profile(&request.location.profile))
                    {
                        self.remote.profile = p.clone();
                    }
                    self.remote.visible = false;
                    self.remote.message=Some("Connected. Remote links, archives and mutations other than Copy are unsupported.".into());
                    self.restore_copy_focus = true;
                }
                Err(e) => {
                    self.remote.message = Some(e);
                    if let Some(tab) = self.remote.target
                        && let Some(p) = self.panes.find_mut(tab)
                    {
                        p.remote_request = None;
                    }
                }
            }
        }
        if let Some(result) = self.remote.prepare.poll() {
            match result {
                Ok(plan) => {
                    if let Some((side, tab)) = self.remote.prepare_origin.take() {
                        self.remote.review = Some((plan, side, tab));
                    }
                }
                Err(e) => self.remote.message = Some(e),
            }
        }
        if let Some(result) = self.remote.download.poll() {
            self.remote.download_profile = None;
            match result {
                Ok((path, view, side, tab)) => {
                    if view {
                        self.view_prepared_remote(path, side, tab, ctx);
                    } else {
                        self.opener.start(crate::opening::Target::Local(path));
                        self.opener.message =
                            Some("Remote copy retained; edits do not update the server.".into());
                    }
                }
                Err(e) => self.remote.message = Some(e),
            }
        }
        if let Some(result) = self.remote.credential.poll() {
            if result.is_ok()
                && let Some(p) = crate::appearance::settings_path()
            {
                self.remote.store = ConnectionStore::load(p.with_file_name("sftp.json")).ok();
            }
            self.remote.message = Some(result.unwrap_or_else(|e| e));
        }
        let requests: Vec<_> = self
            .panes
            .all()
            .filter_map(|t| t.pane.remote_request.clone().map(|r| (t.id, r)))
            .collect();
        for (tab, request) in requests {
            if self.remote.connect.busy() || self.remote.trust.is_some() {
                continue;
            }
            if self
                .remote
                .listings
                .get(&tab)
                .is_some_and(|l| l.request.generation == request.generation)
            {
                continue;
            }
            if let Some(session) = self.remote.session(&request.location.profile) {
                let l = self.remote.listings.entry(tab).or_insert_with(|| Listing {
                    request: request.clone(),
                    job: ReadJob::default(),
                });
                l.request = request.clone();
                let location = request.location.clone();
                if let Err(e) = l
                    .job
                    .start(move |c| session.list(&location, &c).map_err(|e| e.to_string()))
                {
                    self.remote.message = Some(e);
                }
            } else {
                let profile = self
                    .remote
                    .store
                    .as_ref()
                    .and_then(|s| s.profile(&request.location.profile))
                    .cloned();
                if let Some(profile) = profile {
                    self.remote.profile = profile;
                    self.remote.target = Some(tab);
                    self.remote.visible = true;
                    clear_secret(&mut self.remote.secret);
                    self.remote.use_saved = false;
                    self.remote.remember = false;
                    if let Some(p) = self.panes.find_mut(tab) {
                        p.remote_request = None;
                        p.error = Some(
                            "Connect this saved SFTP profile to open the requested location".into(),
                        );
                    }
                    self.remote.resume = Some(request.clone());
                    self.remote.profile.directory = request.location.path;
                } else if let Some(p) = self.panes.find_mut(tab) {
                    p.remote_request = None;
                    p.error=Some("SFTP profile missing. Edit the bookmark or create the profile; previous location retained.".into());
                }
            }
        }
        for (tab, l) in &mut self.remote.listings {
            if !self.panes.contains(*tab) {
                l.job.discard();
            }
            if let Some(result) = l.job.poll()
                && let Some(pane) = self.panes.find_mut(*tab)
                && pane
                    .remote_request
                    .as_ref()
                    .is_some_and(|r| r.generation == l.request.generation)
            {
                match result {
                    Ok(entries) => {
                        pane.remote_loaded(&l.request, entries, &self.sorter);
                    }
                    Err(e) => {
                        pane.remote_request = None;
                        pane.error = Some(e);
                    }
                }
            }
        }
        self.remote
            .listings
            .retain(|tab, l| self.panes.contains(*tab) || l.job.busy());
        if self.copy_job.is_none() {
            self.remote.transfer_profiles.clear();
        }
        if self.remote.busy() {
            ctx.request_repaint_after(Duration::from_millis(30));
        }
    }
    pub(super) fn remote_window(&mut self, ctx: &egui::Context) {
        let mut connect = false;
        let mut save = false;
        let mut remove = false;
        let mut disconnect = false;
        let mut cancel = false;
        if self.remote.visible {
            let mut visible = true;
            egui::Window::new("SFTP connections").id(egui::Id::new("sftp-connections")).open(&mut visible).default_width(650.).show(ctx,|ui|{let r=&mut self.remote;
   ui.horizontal_wrapped(|ui|{if let Some(store)=&r.store{for p in store.profiles(){if ui.selectable_label(r.profile.id==p.id,&p.label).clicked(){r.profile=p.clone();clear_secret(&mut r.secret);r.remember=false;r.use_saved=false;}}}
if ui.button("New profile").clicked(){r.profile.id=r.store.as_ref().map_or_else(||"server-1".into(),ConnectionStore::unused_profile_id);r.profile.label="SFTP server".into();clear_secret(&mut r.secret);r.use_saved=false;r.remember=false;}});
   egui::Grid::new("sftp-fields").num_columns(2).show(ui,|ui|{for (label,value) in [("Profile ID",&mut r.profile.id),("Label",&mut r.profile.label),("Host (no URL)",&mut r.profile.host),("User",&mut r.profile.user),("Initial directory",&mut r.profile.directory)]{ui.label(label);ui.add(egui::TextEdit::singleline(value).desired_width(380.));ui.end_row();}ui.label("Port");ui.add(egui::DragValue::new(&mut r.profile.port).range(1..=65535));ui.end_row();});
   ui.horizontal(|ui|{let mut key=matches!(r.profile.authentication,Authentication::PrivateKey(_));if ui.checkbox(&mut key,"Private-key file authentication").changed(){r.profile.authentication=if key{Authentication::PrivateKey(PathBuf::new())}else{Authentication::Password};}
if let Authentication::PrivateKey(path)=&mut r.profile.authentication{let mut text=path.display().to_string();if ui.text_edit_singleline(&mut text).changed(){*path=PathBuf::from(text);}}});
   ui.horizontal(|ui|{ui.label("Password / key passphrase");ui.add(egui::TextEdit::singleline(&mut r.secret).password(true).desired_width(300.));});
   ui.checkbox(&mut r.use_saved,"Use saved native credential");ui.checkbox(&mut r.remember,"Remember secret in OS credential store (explicit opt-in)");ui.small("If Keychain/Secret Service is unavailable, remembering fails visibly. Uncheck Remember to connect for this session. Secrets are never saved in profiles.");
   ui.horizontal(|ui|{save=ui.add_enabled(r.store.is_some()&&!r.credential.busy(),egui::Button::new("Save profile")).clicked();remove=ui.add_enabled(r.store.as_ref().is_some_and(|s|s.profile(&r.profile.id).is_some())&&!r.credential.busy(),egui::Button::new("Delete profile…")).clicked();connect=ui.add_enabled(!r.connect.busy()&&!r.modal(),egui::Button::new("Connect")).clicked();disconnect=ui.add_enabled(r.session(&r.profile.id).is_some(),egui::Button::new("Disconnect")).clicked();cancel=ui.button("Cancel pending").clicked();});
   if r.connect.busy()||r.download.busy()||r.prepare.busy(){ui.spinner();ui.label("Network preparation running; cancellation waits for bounded calls.");}
   ui.collapsing("Trusted hosts",|ui|{if let Some(store)=&r.store{for key in store.trusted(){ui.horizontal(|ui|{ui.label(format!("{} · {} · {}",key.endpoint,key.algorithm,key.fingerprint));if ui.button("Forget trust…").clicked(){r.forget=Some(key.endpoint.clone());}});}}});
   if let Some(message)=&r.message{ui.label(message);}
  });
            if !visible {
                self.remote.visible = false;
                clear_secret(&mut self.remote.secret);
            }
        }
        if save {
            let mut p = self.remote.profile.clone();
            p.remembered = self
                .remote
                .store
                .as_ref()
                .and_then(|s| s.profile(&p.id))
                .is_some_and(|p| p.remembered);
            p.credential_context = self
                .remote
                .store
                .as_ref()
                .and_then(|s| s.profile(&p.id))
                .and_then(|p| p.credential_context.clone());
            let disconnect = self.remote.sessions.get(&p.id).is_some_and(|s| {
                s.profile.endpoint() != p.endpoint()
                    || s.profile.user != p.user
                    || s.profile.authentication != p.authentication
            });
            let result = self
                .remote
                .store
                .as_mut()
                .ok_or("SFTP configuration unavailable".to_owned())
                .and_then(|s| s.save_profile(p.clone()).map_err(|e| e.to_string()));
            match result {
                Ok(()) => {
                    self.remote.profile = p.clone();
                    if disconnect {
                        self.remote.use_saved = false;
                        self.remote.remember = false;
                        clear_secret(&mut self.remote.secret);
                        self.disconnect_remote(&p.id);
                    }
                    self.remote.message=Some("Saved non-secret profile; changed endpoints require a fresh verified connection.".into());
                }
                Err(e) => self.remote.message = Some(e),
            }
        }
        if remove {
            self.remote.forget = Some(format!("profile:{}", self.remote.profile.id));
        }
        if disconnect {
            let id = self.remote.profile.id.clone();
            self.disconnect_remote(&id);
        }
        if cancel {
            self.remote.connect.discard();
            self.remote.prepare.discard();
            self.remote.download.discard();
            self.remote.trust = None;
            for t in self.panes.all_mut() {
                if t.pane.remote_request.is_some() {
                    t.pane.remote_request = None;
                }
            }
            clear_secret(&mut self.remote.secret);
            self.remote.message = Some("Preparation cancelled".into());
        }
        if connect {
            self.begin_connection();
        }
        if let Some((handshake, _)) = &self.remote.trust {
            let key = handshake.identity.clone();
            let mut accept = false;
            let mut decline = false;
            egui::Modal::new(egui::Id::new("sftp-first-trust")).show(ctx,|ui|{ui.set_width(600.);ui.heading("Trust this SFTP host key?");ui.label(&key.endpoint);ui.label(&key.algorithm);ui.monospace(&key.fingerprint);ui.label("Verify this fingerprint with the server owner before trusting. Authentication has not been sent.");ui.horizontal(|ui|{decline=ui.button("Cancel").clicked();accept=ui.button("Trust host and connect").clicked();});});
            if decline {
                self.remote.trust = None;
                self.remote.message =
                    Some("Host trust declined; no authentication or trust saved".into());
                if let Some(tab) = self.remote.target
                    && let Some(p) = self.panes.find_mut(tab)
                {
                    p.remote_request = None;
                }
            }
            if accept {
                let result = self
                    .remote
                    .store
                    .as_mut()
                    .ok_or("SFTP config unavailable".into())
                    .and_then(|s| s.trust(key).map_err(|e| e.to_string()));
                match result {
                    Ok(()) => {
                        let (h, i) = self.remote.trust.take().expect("trust decision");
                        self.remote.authenticate(h, i);
                    }
                    Err(e) => {
                        self.remote.trust = None;
                        self.remote.message = Some(e);
                    }
                }
            }
        }
        if let Some(value) = self.remote.forget.clone() {
            let mut yes = false;
            let mut no = false;
            egui::Modal::new(egui::Id::new("sftp-forget")).show(ctx,|ui|{ui.heading(if value.starts_with("profile:"){"Delete profile and its saved credential?"}else{"Forget this trusted host key?"});ui.label(&value);ui.label("Existing connections will be disconnected. User private-key files are never removed.");ui.horizontal(|ui|{no=ui.button("Cancel").clicked();yes=ui.button("Confirm removal").clicked();});});
            if no {
                self.remote.forget = None;
            }
            if yes {
                self.remote.forget = None;
                if let Some(id) = value.strip_prefix("profile:") {
                    let id = id.to_owned();
                    let remembered = self
                        .remote
                        .store
                        .as_ref()
                        .and_then(|s| s.profile(&id))
                        .is_some_and(|p| p.remembered);
                    self.disconnect_remote(&id);
                    let path =
                        crate::appearance::settings_path().map(|p| p.with_file_name("sftp.json"));
                    self.remote
                        .credential
                        .start(move |_| {
                            if remembered {
                                delete_secret(&id).map_err(|e| e.to_string())?;
                            }
                            let mut store =
                                ConnectionStore::load(path.ok_or("Configuration unavailable")?)
                                    .map_err(|e| e.to_string())?;
                            store.remove_profile(&id).map_err(|e| e.to_string())?;
                            Ok("Profile and native credential removed; reload connections".into())
                        })
                        .unwrap_or_else(|e| self.remote.message = Some(e));
                } else {
                    let ids: Vec<_> = self
                        .remote
                        .sessions
                        .iter()
                        .filter(|(_, s)| s.profile.endpoint() == value)
                        .map(|(id, _)| id.clone())
                        .collect();
                    for id in ids {
                        self.disconnect_remote(&id);
                    }
                    if let Some(s) = &mut self.remote.store
                        && let Err(e) = s.forget_host(&value)
                    {
                        self.remote.message = Some(e.to_string());
                    }
                }
            }
        }
        self.remote_review(ctx);
    }
    fn remote_review(&mut self, ctx: &egui::Context) {
        let Some((plan, _, _)) = &mut self.remote.review else {
            return;
        };
        let mut apply = false;
        let mut cancel = false;
        egui::Modal::new(egui::Id::new("sftp-copy-review")).show(ctx,|ui|{ui.set_width(650.);ui.heading("Copy between local folder and SFTP");ui.label(format!("From: {}",plan.source.description()));ui.label(format!("To: {}",plan.destination.description()));ui.label(plan.names().join(", "));ui.horizontal(|ui|{ui.selectable_value(&mut plan.policy,CopyPolicy::Skip,"Skip existing");ui.selectable_value(&mut plan.policy,CopyPolicy::Overwrite,"Overwrite");});ui.label("Hidden descendants included; links/special files are rejected. Remote identity uses kind/size/second-resolution time. Owned staging paths may remain after failure or cancellation; sources and earlier completed work are retained.");ui.label("Overwrite requires advertised POSIX rename. A failed/uncertain publication is not retried automatically. No remote move or writeback.");ui.horizontal(|ui|{cancel=ui.button("Cancel").clicked();apply=ui.add_enabled(self.copy_job.is_none(),egui::Button::new("Apply copy")).clicked();});});
        if cancel {
            self.remote.review = None;
        } else if apply {
            let (plan, side, tab) = self.remote.review.take().expect("reviewed");
            let source_remote = match &plan.source {
                Endpoint::Remote { location, .. } => Some(location.clone()),
                _ => None,
            };
            let directory = match &plan.source {
                Endpoint::Local(p) => p.clone(),
                _ => PathBuf::new(),
            };
            self.remote.transfer_profiles = [plan.source.profile(), plan.destination.profile()]
                .into_iter()
                .flatten()
                .map(str::to_owned)
                .collect();
            self.start_job(FileAction::RemoteCopy,side,tab,directory,None,move|c|plan.apply(&c).map(|r|JobResult{cancelled:r.cancelled,outcomes:r.outcomes,issues:r.issues,summary:"SFTP copy finished; inspect partial paths in Details. Remote metadata identity is weaker than local inode identity.".into()}).map_err(|e|e.to_string()));
            if let Some(job) = &mut self.copy_job {
                job.source_remote = source_remote;
            }
        }
    }
}

impl Drop for RemoteUi {
    fn drop(&mut self) {
        clear_secret(&mut self.secret);
        for session in self.sessions.values() {
            session.disconnect();
        }
    }
}

fn continue_connection_drop(handshake: Handshake, input: Inputs) {
    drop(handshake);
    drop(input);
}
