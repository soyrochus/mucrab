// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

import com.google.gson.Gson;
import com.mucommander.commons.file.AbstractFile;
import com.mucommander.commons.file.FileFactory;
import com.mucommander.conf.MuConfigurations;
import com.mucommander.conf.MuPreference;
import com.mucommander.text.CustomDateFormat;
import com.mucommander.text.Translator;
import com.mucommander.ui.action.MuAction;
import com.mucommander.ui.main.MainFrame;
import com.mucommander.ui.main.table.FileTable;
import com.mucommander.ui.main.table.FileTableModel;
import com.mucommander.ui.main.table.SortInfo;
import java.awt.event.InputEvent;
import java.awt.event.MouseEvent;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import javax.swing.AbstractAction;
import javax.swing.JTable;
import sun.reflect.ReflectionFactory;

/** Calls original binary methods; reflection only supplies isolated UI wiring. */
public final class W02Oracle {
    static final class Event {
        String op; int row; boolean ctrl; boolean shift;
        boolean ascending; boolean directories_first; String mode;
    }
    static final class Request {
        String id; String path; List<String> names; boolean parent; List<Event> events;
    }
    static final class Snapshot {
        List<String> visible = new ArrayList<>();
        Integer focus_row; String focused;
        List<Boolean> marks = new ArrayList<>();
        List<String> marked = new ArrayList<>();
        List<String> selected = new ArrayList<>();
        int marked_count;
    }
    static Object invoke(Object target, Class<?> type, String name, Class<?>[] types, Object... args) throws Exception {
        Method method = type.getDeclaredMethod(name, types);
        method.setAccessible(true); return method.invoke(target, args);
    }
    static void field(Object target, Class<?> type, String name, Object value) throws Exception {
        Field field = type.getDeclaredField(name); field.setAccessible(true); field.set(target, value);
    }
    static Object allocate(Class<?> type, Class<?> base) throws Exception {
        return ReflectionFactory.getReflectionFactory().newConstructorForSerialization(type, base.getDeclaredConstructor()).newInstance();
    }
    static void action(MainFrame frame, String name) throws Exception {
        Class<?> type = Class.forName("com.mucommander.ui.action.impl." + name + "Action");
        MuAction action = (MuAction) allocate(type, AbstractAction.class);
        field(action, MuAction.class, "mainFrame", frame);
        if (name.equals("MarkAll"))
            field(action, Class.forName("com.mucommander.ui.action.impl.MarkAllAction"), "mark", true);
        action.performAction();
    }
    static Snapshot snapshot(FileTable table, FileTableModel model) {
        Snapshot state = new Snapshot();
        if (model.getRowCount() > 0) {
            state.focus_row = table.getSelectedRow();
            state.focused = table.getFileNameAtRow(state.focus_row);
        }
        for (int i = 0; i < model.getRowCount(); i++) {
            state.visible.add(table.getFileNameAtRow(i));
            state.marks.add(model.isRowMarked(i));
        }
        for (AbstractFile file : model.getMarkedFiles()) state.marked.add(file.getName());
        for (AbstractFile file : table.getSelectedFiles()) state.selected.add(file.getName());
        state.marked_count = model.getNbMarkedFiles();
        return state;
    }
    static Object run(Request request) throws Exception {
        FileTableModel model = new FileTableModel();
        SortInfo sort = new SortInfo();
        invoke(sort, SortInfo.class, "setAscendingOrder", new Class[]{boolean.class}, true);
        invoke(sort, SortInfo.class, "setFoldersFirst", new Class[]{boolean.class}, true);
        invoke(model, FileTableModel.class, "setSortInfo", new Class[]{SortInfo.class}, sort);
        invoke(model, FileTableModel.class, "setFilenameLocale", new Class[]{Locale.class}, Locale.US);
        invoke(model, FileTableModel.class, "setShowParentFolder", new Class[]{boolean.class}, request.parent);
        MuConfigurations.getPreferences().setVariable(MuPreference.FILE_COMPARATOR_USE_LEXICOGRAPHIC_SORT, false);
        AbstractFile folder = FileFactory.getFile(request.path);
        AbstractFile[] children = new AbstractFile[request.names.size()];
        for (int i = 0; i < children.length; i++) children[i] = FileFactory.getFile(request.path + "/" + request.names.get(i));
        invoke(model, FileTableModel.class, "setCurrentFolder", new Class[]{AbstractFile.class, AbstractFile[].class}, folder, children);
        invoke(model, FileTableModel.class, "sortRows", new Class[]{});
        FileTable table = (FileTable) allocate(FileTable.class, JTable.class);
        field(table, FileTable.class, "tableModel", model);
        field(table, FileTable.class, "sortInfo", sort);
        field(table, FileTable.class, "tableSelectionListeners", new WeakHashMap<>());
        table.setModel(model); table.getSelectionModel().setSelectionMode(0);
        table.setSize(1000, 1000);
        MainFrame frame = (MainFrame) allocate(MainFrame.class, Object.class);
        field(frame, MainFrame.class, "activeTable", table);
        field(table, FileTable.class, "mainFrame", frame);
        if (model.getRowCount() > 0) table.selectRow(0);
        List<Snapshot> states = new ArrayList<>(); states.add(snapshot(table, model));
        for (Event event : request.events) {
            if (model.getRowCount() == 0) throw new IllegalArgumentException("Events require rows");
            switch (event.op) {
                case "focus": case "click":
                    if (event.row < 0 || event.row >= model.getRowCount()) throw new IllegalArgumentException("Invalid row");
                    // Focus is normalized independently of the platform's JTable
                    // menu-shortcut toggle (Command on macOS, Control elsewhere).
                    table.changeSelection(event.row, 0, false, false);
                    if (event.op.equals("click")) {
                        int modifiers = (event.ctrl ? InputEvent.CTRL_DOWN_MASK : 0) | (event.shift ? InputEvent.SHIFT_DOWN_MASK : 0);
                        table.mousePressed(new MouseEvent(table, MouseEvent.MOUSE_PRESSED, System.currentTimeMillis(), modifiers,
                            20, event.row * table.getRowHeight() + 2, 1, false, MouseEvent.BUTTON1));
                    }
                    break;
                case "space": table.markSelectedFile(); break;
                case "release":
                    field(table, FileTable.class, "markKeyRepeated", false);
                    field(table, FileTable.class, "lastRowMarked", false); break;
                case "next": action(frame, "SelectNextRow"); break;
                case "previous": action(frame, "SelectPreviousRow"); break;
                case "first": action(frame, "SelectFirstRow"); break;
                case "last": action(frame, "SelectLastRow"); break;
                case "mark_next": action(frame, "MarkNextRow"); break;
                case "mark_previous": action(frame, "MarkPreviousRow"); break;
                case "all": action(frame, "MarkAll"); break;
                case "none": action(frame, "UnmarkAll"); break;
                case "invert": action(frame, "InvertSelection"); break;
                case "sort":
                    invoke(sort, SortInfo.class, "setAscendingOrder", new Class[]{boolean.class}, event.ascending);
                    invoke(sort, SortInfo.class, "setFoldersFirst", new Class[]{boolean.class}, event.directories_first);
                    MuConfigurations.getPreferences().setVariable(MuPreference.FILE_COMPARATOR_USE_LEXICOGRAPHIC_SORT, event.mode.equals("LEXICOGRAPHIC"));
                    invoke(table, FileTable.class, "sortTable", new Class[]{}); break;
                default: throw new IllegalArgumentException("Unknown event " + event.op);
            }
            states.add(snapshot(table, model));
        }
        return Map.of("id", request.id, "status", "ok", "states", states);
    }
    public static void main(String[] args) throws Exception {
        PrintStream protocol = System.out; System.setOut(System.err);
        ResourceBundle dictionary = ResourceBundle.getBundle("dictionary", Locale.ENGLISH);
        Translator.init(dictionary, dictionary, List.of(Locale.US)); CustomDateFormat.init();
        Gson gson = new com.google.gson.GsonBuilder().serializeNulls().create();
        try (BufferedReader input = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8))) {
            String line;
            while ((line = input.readLine()) != null) protocol.println(gson.toJson(run(gson.fromJson(line, Request.class))));
        }
    }
}
