// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

import com.google.gson.*;
import com.mucommander.commons.file.*;
import com.mucommander.commons.file.util.FileComparator;
import com.mucommander.conf.*;
import com.mucommander.text.*;
import com.mucommander.ui.main.MainFrame;
import com.mucommander.ui.main.table.*;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import javax.swing.*;

/** Original comparator/model/table methods; synthetic overrides supply only fixture fields. */
public final class W02cOracle {
    static final class Metadata { long size; long modified_ms; String extension; }
    static final class Entry {
        String name; boolean is_directory; boolean is_symlink; String path; Metadata metadata;
    }
    static final class Request {
        String id; String kind; String path; String mode; boolean parent; boolean pairs_only;
        List<Entry> entries; List<JsonObject> events;
    }
    static final class MetadataFile extends ProxyFile {
        final Entry data;
        MetadataFile(AbstractFile backing, Entry data) { super(backing); this.data = data; }
        @Override public String getName() { return data.name; }
        @Override public long getSize() { return data.metadata.size; }
        @Override public long getDate() { return data.metadata.modified_ms; }
        @Override public String getExtension() { return data.metadata.extension; }
    }
    static Object get(Object target, Class<?> type, String name) throws Exception {
        Field f = type.getDeclaredField(name); f.setAccessible(true); return f.get(target);
    }
    static Map<String,Object> describe(AbstractFile file) {
        Metadata metadata = new Metadata(); metadata.size = file.getSize(); metadata.modified_ms = file.getDate(); metadata.extension = file.getExtension();
        return Map.of("name", file.getName(), "is_directory", file.isDirectory(), "is_symlink", file.isSymlink(), "metadata", metadata);
    }
    static Object snapshot(FileTable table, FileTableModel model, SortInfo sort, String mode, boolean pairsOnly) throws Exception {
        FileComparator comparator = (FileComparator) W02Oracle.invoke(model, FileTableModel.class, "getFileComparator", new Class[]{SortInfo.class, Locale.class}, sort, Locale.US);
        AbstractFile[] files = (AbstractFile[]) get(model, FileTableModel.class, "cachedFiles");
        int[][] pairs = new int[files.length][files.length];
        for (int i = 0; i < files.length; i++) for (int j = 0; j < files.length; j++) pairs[i][j] = Integer.signum(comparator.compare(files[i], files[j]));
        Map<String,Object> state = new LinkedHashMap<>();
        state.put("criterion", sort.getCriterion().toString()); state.put("ascending", sort.getAscendingOrder());
        state.put("directories_first", sort.getFoldersFirst()); state.put("mode", mode);
        state.put("pairs", pairs); state.put("order", pairsOnly ? null : ((int[]) get(model, FileTableModel.class, "fileArrayIndex")).clone());
        state.put("selection", pairsOnly ? null : W02Oracle.snapshot(table, model)); return state;
    }
    static Object run(Request request) throws Exception {
        if (!List.of("NATURAL", "LEXICOGRAPHIC").contains(request.mode)) throw new IllegalArgumentException("Invalid mode");
        MuConfigurations.getPreferences().setVariable(MuPreference.FILE_COMPARATOR_USE_LEXICOGRAPHIC_SORT, request.mode.equals("LEXICOGRAPHIC"));
        AbstractFile folder = FileFactory.getFile(request.path);
        AbstractFile[] files;
        if (request.kind.equals("directory")) files = folder.ls();
        else if (request.kind.equals("synthetic")) {
            files = new AbstractFile[request.entries.size()];
            for (int i = 0; i < files.length; i++) {
                Entry e = request.entries.get(i); AbstractFile backing = FileFactory.getFile(e.path);
                if (!backing.exists() || backing.isDirectory() != e.is_directory) throw new IllegalArgumentException("Synthetic backing changed");
                files[i] = new MetadataFile(backing, e);
            }
        } else throw new IllegalArgumentException("Unknown fixture layer");
        List<Map<String,Object>> observed = Arrays.stream(files).map(W02cOracle::describe).toList();
        SortInfo sort = new SortInfo(); FileTableModel model = new FileTableModel();
        W02Oracle.invoke(sort, SortInfo.class, "setAscendingOrder", new Class[]{boolean.class}, true);
        W02Oracle.invoke(sort, SortInfo.class, "setFoldersFirst", new Class[]{boolean.class}, true);
        W02Oracle.invoke(model, FileTableModel.class, "setSortInfo", new Class[]{SortInfo.class}, sort);
        W02Oracle.invoke(model, FileTableModel.class, "setFilenameLocale", new Class[]{Locale.class}, Locale.US);
        W02Oracle.invoke(model, FileTableModel.class, "setShowParentFolder", new Class[]{boolean.class}, request.parent);
        W02Oracle.invoke(model, FileTableModel.class, "setCurrentFolder", new Class[]{AbstractFile.class, AbstractFile[].class}, folder, files);
        W02Oracle.invoke(model, FileTableModel.class, "sortRows", new Class[]{});
        FileTable table = (FileTable) W02Oracle.allocate(FileTable.class, JTable.class);
        W02Oracle.field(table, FileTable.class, "tableModel", model); W02Oracle.field(table, FileTable.class, "sortInfo", sort);
        W02Oracle.field(table, FileTable.class, "tableSelectionListeners", new WeakHashMap<>());
        table.setModel(model); table.getSelectionModel().setSelectionMode(0); table.setSize(1000, 1000);
        MainFrame frame = (MainFrame) W02Oracle.allocate(MainFrame.class, Object.class);
        W02Oracle.field(frame, MainFrame.class, "activeTable", table); W02Oracle.field(table, FileTable.class, "mainFrame", frame);
        if (model.getRowCount() > 0) table.selectRow(0);
        List<Object> states = new ArrayList<>(); states.add(snapshot(table, model, sort, request.mode, request.pairs_only));
        for (JsonObject event : request.events) {
            System.err.println("W02c event: " + request.id + " " + event);
            String op = event.get("op").getAsString();
            if (op.equals("sort")) {
                String criterion = event.get("criterion").getAsString();
                if (!List.of("NAME", "SIZE", "DATE", "EXTENSION").contains(criterion)) throw new IllegalArgumentException("Criterion outside contract");
                W02Oracle.invoke(sort, SortInfo.class, "setCriterion", new Class[]{Column.class}, Column.valueOf(criterion));
                W02Oracle.invoke(sort, SortInfo.class, "setAscendingOrder", new Class[]{boolean.class}, event.get("ascending").getAsBoolean());
                W02Oracle.invoke(sort, SortInfo.class, "setFoldersFirst", new Class[]{boolean.class}, event.get("directories_first").getAsBoolean());
                if (!request.pairs_only) W02Oracle.invoke(table, FileTable.class, "sortTable", new Class[]{});
            } else if (op.equals("focus") || op.equals("mark")) {
                int row = event.get("row").getAsInt();
                if (row < 0 || row >= model.getRowCount()) throw new IllegalArgumentException("Invalid row");
                table.selectRow(row);
                if (op.equals("mark")) table.setRowMarked(row, true);
            } else throw new IllegalArgumentException("Unknown event " + op);
            states.add(snapshot(table, model, sort, request.mode, request.pairs_only));
        }
        return Map.of("id", request.id, "status", "ok", "entries", observed, "states", states);
    }
    public static void main(String[] args) throws Exception {
        PrintStream protocol = System.out; System.setOut(System.err);
        Thread watchdog = new Thread(() -> {
            try { Thread.sleep(15000); } catch (InterruptedException ignored) { return; }
            for (Map.Entry<Thread,StackTraceElement[]> stack : Thread.getAllStackTraces().entrySet()) {
                System.err.println("W02c diagnostic thread: " + stack.getKey().getName());
                for (StackTraceElement element : stack.getValue()) System.err.println("  " + element);
            }
        }); watchdog.setDaemon(true); watchdog.start();
        ResourceBundle dictionary = ResourceBundle.getBundle("dictionary", Locale.ENGLISH);
        Translator.init(dictionary, dictionary, List.of(Locale.US)); CustomDateFormat.init();
        Gson gson = new GsonBuilder().serializeNulls().create();
        try (BufferedReader input = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8))) {
            String line;
            while ((line = input.readLine()) != null) {
                Request request = gson.fromJson(line, Request.class);
                System.err.println("W02c start: " + request.id);
                protocol.println(gson.toJson(run(request)));
                System.err.println("W02c complete: " + request.id);
            }
        }
    }
}
