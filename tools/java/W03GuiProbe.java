// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
import java.awt.*;
import java.awt.image.BufferedImage;
import java.lang.instrument.Instrumentation;
import java.lang.reflect.*;
import java.nio.file.*;
import java.util.*;
import javax.imageio.ImageIO;
import javax.swing.*;

/** Operates actual CopyDialog controls and observes original full-job completion. */
public final class W03GuiProbe {
    public static void premain(String destination, Instrumentation unused) {
        W02bGuiProbe.output=Path.of(destination);
        new Thread(() -> {
            try { probe(); System.exit(0); }
            catch(Throwable error) {error.printStackTrace();System.exit(2);}
        },"W03-original-GUI-probe").start();
    }
    static Object field(Object target,String name) throws Exception {
        for(Class<?> owner=target.getClass();owner!=null;owner=owner.getSuperclass()) {
            try {Field field=owner.getDeclaredField(name);field.setAccessible(true);return field.get(target);}
            catch(NoSuchFieldException ignored) {}
        }
        throw new NoSuchFieldException(name);
    }
    static void mark(int side,String focus,String... marked) throws Exception {
        JTable table=W02bGuiProbe.tables.get(side);
        java.util.List<String> names=W02bGuiProbe.names(table);
        for(String name:marked) W02bGuiProbe.invoke(table,"setRowMarked",new Class[]{int.class,boolean.class},names.indexOf(name),true);
        W02bGuiProbe.invoke(table,"selectRow",new Class[]{int.class},names.indexOf(focus));
    }
    static java.util.List<String> marked(JTable table) throws Exception {
        java.util.List<String> names=W02bGuiProbe.names(table),marked=new ArrayList<>();
        for(int row=0;row<names.size();row++)if((Boolean)W02bGuiProbe.invoke(table.getModel(),"isRowMarked",new Class[]{int.class},row))marked.add(names.get(row));
        return marked;
    }
    static void copy(int side,String policy,String label) throws Exception {
        SwingUtilities.invokeLater(() -> {
            try {
                JTable table=W02bGuiProbe.tables.get(side);Object frame=field(table,"mainFrame");
                Method active=frame.getClass().getDeclaredMethod("setActiveTable",table.getClass());active.setAccessible(true);active.invoke(frame,table);
                Object files=W02bGuiProbe.invoke(table,"getSelectedFiles",new Class[]{});
                Class<?> dialogClass=table.getClass().getClassLoader().loadClass("com.mucommander.ui.dialog.file.CopyDialog");
                Object dialog=dialogClass.getConstructor(frame.getClass(),files.getClass()).newInstance(frame,files);
                W02bGuiProbe.invoke(dialog,"showDialog",new Class[]{});
            } catch(Exception error) {error.printStackTrace();System.exit(2);}
        });
        JDialog[] found={null};
        for(int attempt=0;attempt<150;attempt++) {
            SwingUtilities.invokeAndWait(() -> {
                for(Window window:Window.getWindows())if(window.isShowing() && window.getClass().getName().equals("com.mucommander.ui.dialog.file.CopyDialog"))found[0]=(JDialog)window;
            });
            if(found[0]!=null)break;Thread.sleep(100);
        }
        if(found[0]==null)throw new IllegalStateException("Original CopyDialog did not appear");
        JDialog dialog=found[0];
        SwingUtilities.invokeAndWait(() -> {
            try {
                JTable other=W02bGuiProbe.tables.get(1-side);
                Object panel=field(other,"folderPanel");Object folder=W02bGuiProbe.invoke(panel,"getCurrentFolder",new Class[]{});
                String path=(String)W02bGuiProbe.invoke(folder,"getAbsolutePath",new Class[]{boolean.class},true);
                ((JTextField)field(dialog,"pathField")).setText(path);
                JComboBox<?> combo=(JComboBox<?>)field(dialog,"fileExistsActionComboBox");
                for(int i=0;i<combo.getItemCount();i++)if(combo.getItemAt(i) instanceof Enum<?> value && value.name().equals(policy))combo.setSelectedIndex(i);
                ((JCheckBox)field(dialog,"skipErrorsCheckBox")).setSelected(true);
                ((JCheckBox)field(dialog,"verifyIntegrityCheckBox")).setSelected(false);
            } catch(Exception error) {throw new RuntimeException(error);}
        });
        boolean[] ready={false};
        for(int attempt=0;attempt<100;attempt++) {
            SwingUtilities.invokeAndWait(() -> {try {ready[0]=((JButton)field(dialog,"okButton")).isEnabled();}catch(Exception error){throw new RuntimeException(error);}});
            if(ready[0])break;Thread.sleep(100);
        }
        if(!ready[0])throw new IllegalStateException("Original copy destination did not resolve");
        Thread.sleep(300);W02bGuiProbe.capture(label);
        SwingUtilities.invokeAndWait(() -> {
            try {
                BufferedImage rendered=new BufferedImage(dialog.getWidth(),dialog.getHeight(),BufferedImage.TYPE_INT_RGB);
                Graphics2D graphics=rendered.createGraphics();dialog.printAll(graphics);graphics.dispose();
                ImageIO.write(rendered,"png",W02bGuiProbe.output.resolve(label+"-dialog.png").toFile());
                ((JButton)field(dialog,"okButton")).doClick();
            } catch(Exception error){throw new RuntimeException(error);}
        });
    }
    static void awaitStage(int stage) throws Exception {
        for(int attempt=0;attempt<150;attempt++) {
            boolean[] ready={false};
            SwingUtilities.invokeAndWait(() -> {
                try {
                    JTable left=W02bGuiProbe.tables.get(0),right=W02bGuiProbe.tables.get(1);
                    ready[0]=switch(stage) {
                        case 1 -> W02bGuiProbe.names(right).containsAll(java.util.List.of("package","send.txt")) && marked(left).equals(java.util.List.of("collision.txt"));
                        case 2 -> W02bGuiProbe.names(left).contains("reply.txt") && marked(right).isEmpty();
                        case 3 -> marked(left).isEmpty();
                        default -> false;
                    };
                }catch(Exception error){throw new RuntimeException(error);}
            });
            if(ready[0]){Thread.sleep(500);return;}Thread.sleep(100);
        }
        throw new IllegalStateException("Original copy job/refresh did not reach stage "+stage);
    }
    static void probe() throws Exception {
        Files.createDirectories(W02bGuiProbe.output);
        for(int attempt=0;attempt<150;attempt++) {
            SwingUtilities.invokeAndWait(() -> {
                for(Window window:Window.getWindows())if(window instanceof Dialog dialog && dialog.isShowing() && dialog.getTitle().contains("Welcome"))W02bGuiProbe.dismissWelcome(dialog);
                W02bGuiProbe.tables.clear();W02bGuiProbe.hiddenToggle=null;
                for(Frame frame:Frame.getFrames())if(frame.isShowing() && frame instanceof JFrame jf){W02bGuiProbe.walk(jf.getContentPane());if(jf.getJMenuBar()!=null)W02bGuiProbe.walk(jf.getJMenuBar());}
            });
            if(W02bGuiProbe.tables.size()==2 && W02bGuiProbe.tables.get(0).getRowCount()>2)break;Thread.sleep(200);
        }
        if(W02bGuiProbe.tables.size()!=2)throw new IllegalStateException("Missing original tables");
        Thread.sleep(500);W02bGuiProbe.capture("original-initial");
        SwingUtilities.invokeAndWait(() -> {try {mark(0,"package","package","send.txt","collision.txt");mark(1,"reply.txt","reply.txt");}catch(Exception error){throw new RuntimeException(error);}});
        W02bGuiProbe.capture("original-marked");
        copy(0,"SKIP","original-forward-review");awaitStage(1);W02bGuiProbe.capture("original-forward-skip");
        copy(1,"SKIP","original-reverse-review");awaitStage(2);W02bGuiProbe.capture("original-reverse-complete");
        copy(0,"OVERWRITE","original-overwrite-review");awaitStage(3);W02bGuiProbe.capture("original-overwrite-complete");
        System.out.println("W03 original CopyDialog, full jobs, auto-unmark and refresh completed in both directions");
    }
}
