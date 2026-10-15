// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

import com.google.gson.Gson;
import com.mucommander.commons.file.AbstractFile;
import com.mucommander.commons.file.FileFactory;
import com.mucommander.commons.file.util.FileComparator;
import com.mucommander.text.Translator;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.ResourceBundle;
import java.util.function.Function;

/** Adapter glue only. Both comparison and table sorting execute original JAR code. */
public final class W01Oracle {
    static final class Entry {
        String name;
        boolean is_directory;
        boolean is_symlink;
        String path;
        Entry(String name, boolean directory, boolean symlink) {
            this.name = name; this.is_directory = directory; this.is_symlink = symlink;
        }
    }
    static final class Request {
        String id;
        String kind;
        String path;
        String locale;
        String mode;
        boolean ascending;
        boolean directories_first;
        List<Entry> entries;
    }
    static final class Response {
        String id;
        String status = "ok";
        String error;
        List<Entry> entries = new ArrayList<>();
        int[][] pairs;
        int[] order;
    }

    public static void main(String[] args) throws Exception {
        // Original libraries log to stdout. Keep stdout exclusively JSON lines.
        PrintStream protocol = System.out;
        System.setOut(System.err);
        ResourceBundle dictionary = ResourceBundle.getBundle("dictionary", Locale.ENGLISH);
        Translator.init(dictionary, dictionary, List.of(Locale.US));
        Class<?> modelType = Class.forName("com.mucommander.ui.main.table.FileTableModel");
        Field cached = modelType.getDeclaredField("cachedFiles");
        Field indices = modelType.getDeclaredField("fileArrayIndex");
        Method sort = modelType.getDeclaredMethod("sort", FileComparator.class, int.class, int.class);
        cached.setAccessible(true); indices.setAccessible(true); sort.setAccessible(true);
        Gson gson = new Gson();
        try (BufferedReader input = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8))) {
            String line;
            while ((line = input.readLine()) != null) {
                Request request = gson.fromJson(line, Request.class);
                if (!"en-US".equals(request.locale)) throw new IllegalArgumentException("Only en-US is in W01 scope");
                Response response = new Response(); response.id = request.id;
                AbstractFile[] files;
                Map<AbstractFile, String> labels = new IdentityHashMap<>();
                Function<AbstractFile, String> nameFunc;
                if ("directory".equals(request.kind)) {
                    try {
                        files = FileFactory.getFile(request.path).ls();
                    } catch (java.io.IOException error) {
                        response.status = "error"; response.error = "directory_unavailable";
                        protocol.println(gson.toJson(response)); continue;
                    }
                    nameFunc = AbstractFile::getName;
                } else if ("synthetic".equals(request.kind)) {
                    files = new AbstractFile[request.entries.size()];
                    for (int i = 0; i < files.length; i++) {
                        Entry entry = request.entries.get(i);
                        files[i] = FileFactory.getFile(entry.path);
                        if (!files[i].exists() || files[i].isDirectory() != entry.is_directory)
                            throw new IllegalArgumentException("Synthetic backing fixture changed");
                        labels.put(files[i], entry.name);
                    }
                    nameFunc = labels::get;
                } else throw new IllegalArgumentException("Unknown request kind");
                for (AbstractFile file : files)
                    response.entries.add(new Entry(nameFunc.apply(file), file.isDirectory(), file.isSymlink()));
                FileComparator comparator = new FileComparator(FileComparator.CRITERION.NAME,
                    request.ascending, request.directories_first, nameFunc, Locale.US,
                    FileComparator.Mode.valueOf(request.mode));
                response.pairs = new int[files.length][files.length];
                for (int i = 0; i < files.length; i++)
                    for (int j = 0; j < files.length; j++)
                        response.pairs[i][j] = Integer.signum(comparator.compare(files[i], files[j]));
                response.order = new int[files.length];
                for (int i = 0; i < files.length; i++) response.order[i] = i;
                Object model = modelType.getConstructor().newInstance();
                cached.set(model, files); indices.set(model, response.order);
                sort.invoke(model, comparator, 0, files.length - 1);
                protocol.println(gson.toJson(response));
            }
        }
    }
}
