// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
import com.google.gson.*;
import com.mucommander.commons.file.*;
import com.mucommander.commons.file.util.FileSet;
import com.mucommander.job.FileJobState;
import com.mucommander.job.impl.MoveJob;
import com.mucommander.text.*;
import com.mucommander.ui.dialog.file.FileCollisionDialog.FileCollisionAction;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.util.*;

/** Real MoveJob construction and processFile; no application method overrides. */
public final class W04Oracle {
    static final class Request {
        String id, source_directory, destination_directory, policy, new_name;
        boolean rename_mode;
        List<String> names;
    }
    static Object invoke(Object target, Class<?> owner, String name, Class<?>[] types, Object... args) throws Exception {
        Method method = owner.getDeclaredMethod(name, types); method.setAccessible(true);
        try { return method.invoke(target, args); }
        catch (InvocationTargetException error) { throw new RuntimeException(error.getCause()); }
    }
    static Object run(Request request) throws Exception {
        AbstractFile source = FileFactory.getFile(request.source_directory);
        AbstractFile destination = FileFactory.getFile(request.destination_directory);
        if (!source.isDirectory() || !destination.isDirectory()) throw new IllegalArgumentException("Existing source/destination folders required");
        FileSet files = new FileSet(source);
        for (String name : request.names) {
            if (name.isEmpty() || name.equals(".") || name.equals("..") || name.contains("/")) throw new IllegalArgumentException("Direct child name required");
            files.add(source.getDirectChild(name));
        }
        MoveJob job = new MoveJob(null, null, files, destination, request.new_name, FileCollisionAction.valueOf(request.policy), request.rename_mode);
        job.setAutoSkipErrors(true); job.setIntegrityCheckEnabled(false);
        invoke(job, com.mucommander.job.impl.FileJob.class, "setState", new Class[]{FileJobState.class}, FileJobState.RUNNING);
        List<Object> outcomes = new ArrayList<>();
        for (int i = 0; i < files.size(); i++) {
            AbstractFile file = files.get(i);
            invoke(job, com.mucommander.job.impl.TransferFileJob.class, "nextFile", new Class[]{AbstractFile.class}, file);
            boolean success = (Boolean) invoke(job, MoveJob.class, "processFile", new Class[]{AbstractFile.class, Object.class}, file, null);
            outcomes.add(Map.of("name", request.names.get(i), "success", success));
        }
        return Map.of("id", request.id, "status", "ok", "outcomes", outcomes, "state", job.getState().toString());
    }
    public static void main(String[] ignored) throws Exception {
        PrintStream protocol = System.out; System.setOut(System.err);
        ResourceBundle dictionary = ResourceBundle.getBundle("dictionary", Locale.ENGLISH);
        Translator.init(dictionary, dictionary, List.of(Locale.US)); CustomDateFormat.init();
        com.mucommander.core.desktop.DesktopManager.init(false);
        Gson gson = new GsonBuilder().serializeNulls().create();
        try (BufferedReader input = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8))) {
            String line;
            while ((line = input.readLine()) != null) {
                Request request = gson.fromJson(line, Request.class);
                System.err.println("W04 start: " + request.id);
                protocol.println(gson.toJson(run(request))); protocol.flush();
                System.err.println("W04 complete: " + request.id);
            }
        }
    }
}
