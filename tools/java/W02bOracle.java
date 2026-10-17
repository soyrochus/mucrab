// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

import com.google.gson.*;
import com.mucommander.commons.file.AbstractFile;
import com.mucommander.commons.file.FileFactory;
import com.mucommander.commons.file.util.FileSet;
import com.mucommander.conf.MuConfigurations;
import com.mucommander.conf.MuPreference;
import com.mucommander.text.CustomDateFormat;
import com.mucommander.text.Translator;
import com.mucommander.ui.event.LocationManager;
import com.mucommander.ui.main.ConfigurableFolderFilter;
import com.mucommander.ui.main.FolderPanel;
import com.mucommander.ui.main.MainFrame;
import com.mucommander.ui.main.table.*;
import java.awt.event.*;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import javax.swing.*;

/** Original filter + original reload worker; no behavioral method overrides. */
public final class W02bOracle {
    static final class Pane {
        String path; List<String> names; boolean parent;
    }
    static final class Request {
        String id; String mode; boolean show_hidden; List<Pane> panes; List<JsonObject> events;
    }
    static final class RuntimePane {
        Pane input; AbstractFile folder; FileTableModel model; FileTable table;
        SortInfo sort; MainFrame frame;
    }
    static final Gson GSON = new GsonBuilder().serializeNulls().create();

    static AbstractFile[] children(RuntimePane pane, ConfigurableFolderFilter filter) throws Exception {
        AbstractFile[] files = new AbstractFile[pane.input.names.size()];
        for (int i = 0; i < files.length; i++)
            files[i] = FileFactory.getFile(pane.input.path + "/" + pane.input.names.get(i));
        return filter.filter(files);
    }
    static RuntimePane create(Pane input, ConfigurableFolderFilter filter) throws Exception {
        RuntimePane p = new RuntimePane(); p.input = input; p.folder = FileFactory.getFile(input.path);
        p.model = new FileTableModel(); p.sort = new SortInfo();
        W02Oracle.invoke(p.sort, SortInfo.class, "setAscendingOrder", new Class[]{boolean.class}, true);
        W02Oracle.invoke(p.sort, SortInfo.class, "setFoldersFirst", new Class[]{boolean.class}, true);
        W02Oracle.invoke(p.model, FileTableModel.class, "setSortInfo", new Class[]{SortInfo.class}, p.sort);
        W02Oracle.invoke(p.model, FileTableModel.class, "setFilenameLocale", new Class[]{Locale.class}, Locale.US);
        W02Oracle.invoke(p.model, FileTableModel.class, "setShowParentFolder", new Class[]{boolean.class}, input.parent);
        W02Oracle.invoke(p.model, FileTableModel.class, "setCurrentFolder", new Class[]{AbstractFile.class, AbstractFile[].class}, p.folder, children(p, filter));
        W02Oracle.invoke(p.model, FileTableModel.class, "sortRows", new Class[]{});
        p.table = (FileTable) W02Oracle.allocate(FileTable.class, JTable.class);
        W02Oracle.field(p.table, FileTable.class, "tableModel", p.model);
        W02Oracle.field(p.table, FileTable.class, "sortInfo", p.sort);
        W02Oracle.field(p.table, FileTable.class, "tableSelectionListeners", new WeakHashMap<>());
        p.table.setModel(p.model); p.table.getSelectionModel().setSelectionMode(0);
        p.table.setColumnModel(new FileTableColumnModel(new FileTableConfiguration()));
        p.table.setSize(1000, 1000);
        FolderPanel panel = (FolderPanel) W02Oracle.allocate(FolderPanel.class, Object.class);
        LocationManager location = (LocationManager) W02Oracle.allocate(LocationManager.class, Object.class);
        W02Oracle.field(location, LocationManager.class, "currentFolder", p.folder.toMonitoredFile());
        W02Oracle.field(panel, FolderPanel.class, "locationManager", location);
        W02Oracle.field(p.table, FileTable.class, "folderPanel", panel);
        p.frame = (MainFrame) W02Oracle.allocate(MainFrame.class, Object.class);
        W02Oracle.field(p.frame, MainFrame.class, "activeTable", p.table);
        W02Oracle.field(p.table, FileTable.class, "mainFrame", p.frame);
        if (p.model.getRowCount() > 0) p.table.selectRow(0);
        return p;
    }
    static void reload(RuntimePane p, ConfigurableFolderFilter filter) throws Exception {
        // These are the same-folder inputs captured by original setCurrentFolder.
        // The original worker performs sorting, focus fallback and mark restoration.
        AbstractFile focused = p.table.getSelectedFile();
        FileSet marked = p.model.getMarkedFiles();
        W02Oracle.invoke(p.model, FileTableModel.class, "setShowParentFolder", new Class[]{boolean.class}, p.input.parent);
        Class<?> type = Class.forName("com.mucommander.ui.main.table.FileTable$FolderChangeThread");
        Constructor<?> constructor = type.getDeclaredConstructor(FileTable.class, AbstractFile.class, AbstractFile[].class, FileSet.class, AbstractFile.class);
        constructor.setAccessible(true);
        Runnable worker = (Runnable) constructor.newInstance(p.table, p.folder, children(p, filter), marked, focused);
        SwingUtilities.invokeAndWait(worker);
    }
    static Object snapshot(List<RuntimePane> panes) {
        return Map.of("show_hidden", MuConfigurations.getPreferences().getVariable(MuPreference.SHOW_HIDDEN_FILES, false),
            "panes", panes.stream().map(p -> W02Oracle.snapshot(p.table, p.model)).toList());
    }
    static void selection(RuntimePane p, JsonObject event) throws Exception {
        String op = event.get("op").getAsString();
        if (op.equals("focus_name") || op.equals("click_name")) {
            String name = event.get("name").getAsString(); int row = -1;
            for (int i = 0; i < p.model.getRowCount(); i++) if (p.table.getFileNameAtRow(i).equals(name)) row = i;
            if (row < 0) throw new IllegalArgumentException("Requested name is not visible: " + name);
            p.table.changeSelection(row, 0, false, false);
            if (op.equals("click_name")) {
                int modifiers = (event.has("ctrl") && event.get("ctrl").getAsBoolean() ? InputEvent.CTRL_DOWN_MASK : 0)
                    | (event.has("shift") && event.get("shift").getAsBoolean() ? InputEvent.SHIFT_DOWN_MASK : 0);
                p.table.mousePressed(new MouseEvent(p.table, MouseEvent.MOUSE_PRESSED, 1L, modifiers,
                    20, row * p.table.getRowHeight() + 2, 1, false, MouseEvent.BUTTON1));
            }
            return;
        }
        switch (op) {
            case "space": p.table.markSelectedFile(); break;
            case "release": W02Oracle.field(p.table, FileTable.class, "markKeyRepeated", false);
                W02Oracle.field(p.table, FileTable.class, "lastRowMarked", false); break;
            case "next": W02Oracle.action(p.frame, "SelectNextRow"); break;
            case "previous": W02Oracle.action(p.frame, "SelectPreviousRow"); break;
            case "first": W02Oracle.action(p.frame, "SelectFirstRow"); break;
            case "last": W02Oracle.action(p.frame, "SelectLastRow"); break;
            case "mark_next": W02Oracle.action(p.frame, "MarkNextRow"); break;
            case "mark_previous": W02Oracle.action(p.frame, "MarkPreviousRow"); break;
            case "all": W02Oracle.action(p.frame, "MarkAll"); break;
            case "none": W02Oracle.action(p.frame, "UnmarkAll"); break;
            case "invert": W02Oracle.action(p.frame, "InvertSelection"); break;
            case "sort":
                W02Oracle.invoke(p.sort, SortInfo.class, "setAscendingOrder", new Class[]{boolean.class}, event.get("ascending").getAsBoolean());
                W02Oracle.invoke(p.sort, SortInfo.class, "setFoldersFirst", new Class[]{boolean.class}, event.get("directories_first").getAsBoolean());
                MuConfigurations.getPreferences().setVariable(MuPreference.FILE_COMPARATOR_USE_LEXICOGRAPHIC_SORT, event.get("mode").getAsString().equals("LEXICOGRAPHIC"));
                W02Oracle.invoke(p.table, FileTable.class, "sortTable", new Class[]{}); break;
            default: throw new IllegalArgumentException("Unknown event " + op);
        }
    }
    static Object run(Request request) throws Exception {
        if (request.panes.size() != 2) throw new IllegalArgumentException("Exactly two panes required");
        MuConfigurations.getPreferences().setVariable(MuPreference.SHOW_HIDDEN_FILES, request.show_hidden);
        MuConfigurations.getPreferences().setVariable(MuPreference.SHOW_DS_STORE_FILES, true);
        MuConfigurations.getPreferences().setVariable(MuPreference.SHOW_SYSTEM_FOLDERS, true);
        MuConfigurations.getPreferences().setVariable(MuPreference.FILE_COMPARATOR_USE_LEXICOGRAPHIC_SORT, request.mode.equals("LEXICOGRAPHIC"));
        ConfigurableFolderFilter filter = new ConfigurableFolderFilter();
        List<RuntimePane> panes = new ArrayList<>();
        for (Pane input : request.panes) panes.add(create(input, filter));
        List<Object> states = new ArrayList<>(); states.add(snapshot(panes));
        for (JsonObject event : request.events) {
            String op = event.get("op").getAsString();
            if (op.equals("toggle_hidden")) {
                boolean old = MuConfigurations.getPreferences().getVariable(MuPreference.SHOW_HIDDEN_FILES, false);
                MuConfigurations.getPreferences().setVariable(MuPreference.SHOW_HIDDEN_FILES, !old);
                for (RuntimePane pane : panes) reload(pane, filter);
            } else {
                int index = event.get("pane").getAsInt();
                RuntimePane pane = panes.get(index);
                if (op.equals("sort") && !event.get("mode").getAsString().equals(request.mode))
                    throw new IllegalArgumentException("This suite freezes the shared sort mode per sequence");
                if (op.equals("refresh")) reload(pane, filter);
                else selection(pane, event);
            }
            states.add(snapshot(panes));
        }
        MuConfigurations.removePreferencesListener(filter);
        return Map.of("id", request.id, "status", "ok", "states", states);
    }
    public static void main(String[] args) throws Exception {
        PrintStream protocol = System.out; System.setOut(System.err);
        ResourceBundle dictionary = ResourceBundle.getBundle("dictionary", Locale.ENGLISH);
        Translator.init(dictionary, dictionary, List.of(Locale.US)); CustomDateFormat.init();
        try (BufferedReader input = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8))) {
            String line;
            while ((line = input.readLine()) != null) protocol.println(GSON.toJson(run(GSON.fromJson(line, Request.class))));
        }
    }
}
