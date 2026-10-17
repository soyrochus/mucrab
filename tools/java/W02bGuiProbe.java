// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

import java.awt.*;
import java.awt.image.BufferedImage;
import java.lang.instrument.Instrumentation;
import java.lang.reflect.Method;
import java.nio.file.*;
import java.util.*;
import javax.imageio.ImageIO;
import javax.swing.*;

/** Observe actual original windows/actions without replacing application code. */
public final class W02bGuiProbe {
    static final java.util.List<JTable> tables = new ArrayList<>();
    static final java.util.List<String> observations = new ArrayList<>();
    static AbstractButton hiddenToggle;
    static Path output;

    public static void premain(String options, Instrumentation unused) {
        output = Path.of(options);
        new Thread(() -> {
            try { probe(); System.exit(0); }
            catch (Throwable error) {
                error.printStackTrace();
                try { Files.writeString(output.resolve("failure.txt"), error.toString()); }
                catch (Exception ignored) { }
                System.exit(2);
            }
        }, "W02b-original-GUI-probe").start();
    }
    static void walk(Component component) {
        if (component instanceof JTable && component.getClass().getName().equals("com.mucommander.ui.main.table.FileTable"))
            tables.add((JTable) component);
        if (component instanceof AbstractButton button && button.getText() != null
                && button.getText().toLowerCase(Locale.ROOT).contains("hidden")) hiddenToggle = button;
        if (component instanceof JMenu menu) for (Component child : menu.getMenuComponents()) walk(child);
        else if (component instanceof Container container) for (Component child : container.getComponents()) walk(child);
    }
    static void dismissWelcome(Component component) {
        if (component instanceof JButton button && "OK".equals(button.getText())) button.doClick();
        else if (component instanceof Container container)
            for (Component child : container.getComponents()) dismissWelcome(child);
    }
    static Object invoke(Object target, String name, Class<?>[] types, Object... values) throws Exception {
        Method method = target.getClass().getMethod(name, types);
        return method.invoke(target, values);
    }
    static String quote(String value) {
        return "\"" + value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n") + "\"";
    }
    static java.util.List<String> names(JTable table) throws Exception {
        java.util.List<String> names = new ArrayList<>();
        for (int row = 0; row < table.getRowCount(); row++)
            names.add((String) invoke(table, "getFileNameAtRow", new Class[]{int.class}, row));
        return names;
    }
    static void awaitHidden(boolean shown) throws Exception {
        for (int attempt = 0; attempt < 100; attempt++) {
            boolean[] ready = {true};
            SwingUtilities.invokeAndWait(() -> {
                try {
                    for (JTable table : tables) ready[0] &= names(table).contains(".hidden") == shown;
                } catch (Exception error) { throw new RuntimeException(error); }
            });
            if (ready[0]) { Thread.sleep(300); return; }
            Thread.sleep(100);
        }
        throw new IllegalStateException("Timed out waiting for both original tables to refresh");
    }
    static void capture(String label) throws Exception {
        StringBuilder state = new StringBuilder("{\"stage\":" + quote(label) + ",\"panes\":[");
        SwingUtilities.invokeAndWait(() -> {
            try {
                for (int index = 0; index < tables.size(); index++) {
                    if (index > 0) state.append(',');
                    JTable table = tables.get(index);
                    java.util.List<String> names = names(table);
                    java.util.List<String> marked = new ArrayList<>();
                    for (int row = 0; row < names.size(); row++)
                        if ((Boolean) invoke(table.getModel(), "isRowMarked", new Class[]{int.class}, row)) marked.add(names.get(row));
                    state.append("{\"visible\":[").append(String.join(",", names.stream().map(W02bGuiProbe::quote).toList()))
                        .append("],\"focused\":").append(quote(names.get(table.getSelectedRow())))
                        .append(",\"marked\":[").append(String.join(",", marked.stream().map(W02bGuiProbe::quote).toList())).append("]}");
                }
            } catch (Exception error) { throw new RuntimeException(error); }
        });
        state.append("]}"); observations.add(state.toString());
        Files.writeString(output.resolve("states.json"), "[" + String.join(",\n", observations) + "]\n");
        // Retain both the X screen and a direct render of the original Swing
        // window. Xvfb/AWT screen captures can be black without a window manager.
        BufferedImage screen = new Robot().createScreenCapture(new Rectangle(Toolkit.getDefaultToolkit().getScreenSize()));
        ImageIO.write(screen, "png", output.resolve(label + ".screen.png").toFile());
        SwingUtilities.invokeAndWait(() -> {
            try {
                Window window = SwingUtilities.getWindowAncestor(tables.get(0));
                BufferedImage rendered = new BufferedImage(window.getWidth(), window.getHeight(), BufferedImage.TYPE_INT_RGB);
                Graphics2D graphics = rendered.createGraphics();
                window.printAll(graphics); graphics.dispose();
                ImageIO.write(rendered, "png", output.resolve(label + ".png").toFile());
            } catch (Exception error) { throw new RuntimeException(error); }
        });
    }
    static void toggle(boolean shown) throws Exception {
        SwingUtilities.invokeAndWait(() -> hiddenToggle.doClick());
        awaitHidden(shown);
    }
    static void markAndFocus(JTable table, String hidden, String visible) throws Exception {
        java.util.List<String> names = names(table);
        for (String name : java.util.List.of(hidden, visible))
            invoke(table, "setRowMarked", new Class[]{int.class, boolean.class}, names.indexOf(name), true);
        invoke(table, "selectRow", new Class[]{int.class}, names.indexOf(hidden));
    }
    static void probe() throws Exception {
        Files.createDirectories(output);
        for (int attempt = 0; attempt < 150; attempt++) {
            SwingUtilities.invokeAndWait(() -> {
                for (Window window : Window.getWindows())
                    if (window instanceof Dialog dialog && dialog.isShowing() && dialog.getTitle().contains("Welcome")) dismissWelcome(dialog);
                tables.clear(); hiddenToggle = null;
                for (Frame frame : Frame.getFrames()) if (frame.isShowing() && frame instanceof JFrame jf) {
                    walk(jf.getContentPane());
                    if (jf.getJMenuBar() != null) walk(jf.getJMenuBar());
                }
            });
            if (tables.size() == 2 && hiddenToggle != null && tables.stream().allMatch(t -> t.getRowCount() > 1)) break;
            Thread.sleep(200);
        }
        if (tables.size() != 2 || hiddenToggle == null) throw new IllegalStateException("Could not find both original tables and hidden-file menu action");
        Thread.sleep(500); awaitHidden(false); capture("original-hidden-off");
        toggle(true); capture("original-hidden-on");
        SwingUtilities.invokeAndWait(() -> {
            try {
                markAndFocus(tables.get(0), ".hidden", "file2.txt");
                markAndFocus(tables.get(1), ".hidden-dir", "Alpha.txt");
            } catch (Exception error) { throw new RuntimeException(error); }
        });
        capture("original-hidden-and-visible-marked");
        toggle(false); capture("original-filtered-focus-fallback");
        toggle(true); capture("original-reshown-hidden-marks-cleared");
        System.out.println("W02b original GUI probe completed: both real tables, menu action, five states");
    }
}
