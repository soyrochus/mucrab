// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
import java.awt.*;
import java.lang.instrument.Instrumentation;
import java.nio.file.*;
import java.util.*;
import javax.swing.*;

/** Calls real original table sorting entry points in an unmodified GUI. */
public final class W02cGuiProbe {
    static final java.util.List<String> options = new ArrayList<>();
    public static void premain(String destination, Instrumentation unused) {
        W02bGuiProbe.output = Path.of(destination);
        new Thread(() -> {
            try { probe(); System.exit(0); }
            catch (Throwable error) { error.printStackTrace(); System.exit(2); }
        }, "W02c-original-GUI-probe").start();
    }
    static Object call(Object target, String name) throws Exception {
        return W02bGuiProbe.invoke(target, name, new Class[]{});
    }
    static void sort(int pane, String criterion, boolean ascending, boolean folders) throws Exception {
        JTable table = W02bGuiProbe.tables.get(pane);
        Class<?> column = table.getClass().getClassLoader().loadClass("com.mucommander.ui.main.table.Column");
        Object value = Arrays.stream(column.getEnumConstants()).filter(c -> c.toString().equals(criterion)).findFirst().orElseThrow();
        W02bGuiProbe.invoke(table, "sortBy", new Class[]{column, boolean.class, boolean.class}, value, ascending, folders);
    }
    static void capture(String stage) throws Exception {
        Thread.sleep(300);
        W02bGuiProbe.capture(stage);
        StringBuilder state = new StringBuilder("{\"stage\":" + W02bGuiProbe.quote(stage) + ",\"panes\":[");
        SwingUtilities.invokeAndWait(() -> {
            try {
                for (int i = 0; i < 2; i++) {
                    if (i > 0) state.append(',');
                    Object info = call(W02bGuiProbe.tables.get(i), "getSortInfo");
                    state.append("{\"criterion\":").append(W02bGuiProbe.quote(call(info, "getCriterion").toString()))
                        .append(",\"ascending\":").append(call(info, "getAscendingOrder"))
                        .append(",\"directories_first\":").append(call(info, "getFoldersFirst")).append('}');
                }
            } catch (Exception error) { throw new RuntimeException(error); }
        });
        options.add(state.append("]}").toString());
        Files.writeString(W02bGuiProbe.output.resolve("sort-options.json"), "[" + String.join(",\n", options) + "]\n");
    }
    static void probe() throws Exception {
        Files.createDirectories(W02bGuiProbe.output);
        for (int attempt = 0; attempt < 150; attempt++) {
            SwingUtilities.invokeAndWait(() -> {
                for (Window window : Window.getWindows())
                    if (window instanceof Dialog dialog && dialog.isShowing() && dialog.getTitle().contains("Welcome")) W02bGuiProbe.dismissWelcome(dialog);
                W02bGuiProbe.tables.clear(); W02bGuiProbe.hiddenToggle = null;
                for (Frame frame : Frame.getFrames()) if (frame.isShowing() && frame instanceof JFrame jf) {
                    W02bGuiProbe.walk(jf.getContentPane());
                    if (jf.getJMenuBar() != null) W02bGuiProbe.walk(jf.getJMenuBar());
                }
            });
            if (W02bGuiProbe.tables.size() == 2 && W02bGuiProbe.hiddenToggle != null && W02bGuiProbe.tables.stream().allMatch(t -> t.getRowCount() > 1)) break;
            Thread.sleep(200);
        }
        if (W02bGuiProbe.tables.size() != 2) throw new IllegalStateException("Missing original tables");
        W02bGuiProbe.awaitHidden(false); capture("original-name");
        SwingUtilities.invokeAndWait(() -> {
            try {
                W02bGuiProbe.markAndFocus(W02bGuiProbe.tables.get(0), "file2.txt", "archive.tar.gz");
                W02bGuiProbe.markAndFocus(W02bGuiProbe.tables.get(1), "file10.TXT", "README");
            } catch (Exception error) { throw new RuntimeException(error); }
        });
        capture("original-marked");
        SwingUtilities.invokeAndWait(() -> {
            try { sort(0, "SIZE", true, true); sort(1, "DATE", false, false); }
            catch (Exception error) { throw new RuntimeException(error); }
        });
        capture("original-size-date");
        SwingUtilities.invokeAndWait(() -> {
            try { sort(0, "EXTENSION", true, false); sort(1, "SIZE", false, true); }
            catch (Exception error) { throw new RuntimeException(error); }
        });
        capture("original-extension-size");
        W02bGuiProbe.toggle(true); capture("original-hidden-refresh");
        System.out.println("W02c original GUI probe completed: five stages, independent sort options and preserved selection");
    }
}
