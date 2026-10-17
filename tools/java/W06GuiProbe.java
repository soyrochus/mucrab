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

/** Real folder-change workers, ZIP tables, CopyDialog/UnpackDialog and full jobs. */
public final class W06GuiProbe {
    static Path fixture;
    public static void premain(String output, Instrumentation unused) {
        W02bGuiProbe.output=Path.of(output);fixture=W02bGuiProbe.output.getParent().resolve("original-fixture");
        new Thread(()->{try{probe();System.exit(0);}catch(Throwable error){error.printStackTrace();System.exit(2);}},"W06-original-GUI-probe").start();
    }
    static Object field(Object o,String name)throws Exception{return W03GuiProbe.field(o,name);}
    static void awaitNames(int side,String... wanted)throws Exception{
        for(int i=0;i<150;i++){
            boolean[] ready={false};SwingUtilities.invokeAndWait(()->{try{ready[0]=W02bGuiProbe.names(W02bGuiProbe.tables.get(side)).containsAll(java.util.List.of(wanted));}catch(Exception e){throw new RuntimeException(e);}});
            if(ready[0]){Thread.sleep(400);return;}Thread.sleep(100);
        }throw new IllegalStateException("Original ZIP table did not reach "+Arrays.toString(wanted));
    }
    static void navigate(int side,Path path,String... wanted)throws Exception{
        Object[] worker={null};
        for(int attempt=0;attempt<100&&worker[0]==null;attempt++){
            SwingUtilities.invokeAndWait(()->{try{
                JTable table=W02bGuiProbe.tables.get(side);ClassLoader loader=table.getClass().getClassLoader();
                Class<?> factory=loader.loadClass("com.mucommander.commons.file.FileFactory");Class<?> fileType=loader.loadClass("com.mucommander.commons.file.AbstractFile");
                Object file=factory.getMethod("getFile",String.class).invoke(null,path.toString());
                Object panel=field(table,"folderPanel");worker[0]=W02bGuiProbe.invoke(panel,"tryChangeCurrentFolder",new Class[]{fileType},file);
                System.out.println("W06 navigate "+path+" provider="+file.getClass().getName()+" worker="+worker[0]);
            }catch(Exception e){throw new RuntimeException(e);}});
            if(worker[0]==null)Thread.sleep(100);
        }
        if(worker[0]==null)throw new IllegalStateException("Original folder-change worker stayed busy");
        awaitNames(side,wanted);
    }
    static void awaitFile(Path path,String contents)throws Exception{
        for(int i=0;i<150;i++){if(Files.isRegularFile(path)&&Arrays.equals(Files.readAllBytes(path),contents.getBytes(java.nio.charset.StandardCharsets.UTF_8))){Thread.sleep(700);return;}Thread.sleep(100);}
        throw new IllegalStateException("Original ZIP job did not produce "+path);
    }
    static void unpack(int side,String label)throws Exception{
        SwingUtilities.invokeLater(()->{try{
            JTable table=W02bGuiProbe.tables.get(side);Object frame=field(table,"mainFrame");Method active=frame.getClass().getDeclaredMethod("setActiveTable",table.getClass());active.setAccessible(true);active.invoke(frame,table);
            Object files=W02bGuiProbe.invoke(table,"getSelectedFiles",new Class[]{});Class<?> cls=table.getClass().getClassLoader().loadClass("com.mucommander.ui.dialog.file.UnpackDialog");
            Object dialog=cls.getConstructor(frame.getClass(),files.getClass()).newInstance(frame,files);W02bGuiProbe.invoke(dialog,"showDialog",new Class[]{});
        }catch(Exception e){e.printStackTrace();System.exit(2);}});
        JDialog[] dialog={null};
        for(int i=0;i<150;i++){SwingUtilities.invokeAndWait(()->{for(Window w:Window.getWindows())if(w.isShowing()&&w.getClass().getSimpleName().equals("UnpackDialog"))dialog[0]=(JDialog)w;});if(dialog[0]!=null)break;Thread.sleep(100);}
        if(dialog[0]==null)throw new IllegalStateException("Missing original UnpackDialog");
        SwingUtilities.invokeAndWait(()->{try{
            ((JTextField)field(dialog[0],"pathField")).setText(fixture.resolve("right").toString()+"/");
            JComboBox<?> combo=(JComboBox<?>)field(dialog[0],"fileExistsActionComboBox");for(int i=0;i<combo.getItemCount();i++)if(combo.getItemAt(i) instanceof Enum<?> e&&e.name().equals("OVERWRITE"))combo.setSelectedIndex(i);
            ((JCheckBox)field(dialog[0],"skipErrorsCheckBox")).setSelected(true);((JCheckBox)field(dialog[0],"verifyIntegrityCheckBox")).setSelected(false);
        }catch(Exception e){throw new RuntimeException(e);}});
        boolean[] enabled={false};for(int i=0;i<100;i++){SwingUtilities.invokeAndWait(()->{try{enabled[0]=((JButton)field(dialog[0],"okButton")).isEnabled();}catch(Exception e){throw new RuntimeException(e);}});if(enabled[0])break;Thread.sleep(100);}
        if(!enabled[0])throw new IllegalStateException("Unpack destination did not resolve");
        W02bGuiProbe.capture(label);
        SwingUtilities.invokeAndWait(()->{try{
            BufferedImage image=new BufferedImage(dialog[0].getWidth(),dialog[0].getHeight(),BufferedImage.TYPE_INT_RGB);Graphics2D graphics=image.createGraphics();dialog[0].printAll(graphics);graphics.dispose();ImageIO.write(image,"png",W02bGuiProbe.output.resolve(label+"-dialog.png").toFile());
            ((JButton)field(dialog[0],"okButton")).doClick();
        }catch(Exception e){throw new RuntimeException(e);}});
    }
    static void probe()throws Exception{
        Files.createDirectories(W02bGuiProbe.output);
        for(int i=0;i<150;i++){
            SwingUtilities.invokeAndWait(()->{
                for(Window w:Window.getWindows())if(w instanceof Dialog d&&d.isShowing()&&d.getTitle().contains("Welcome"))W02bGuiProbe.dismissWelcome(d);
                W02bGuiProbe.tables.clear();W02bGuiProbe.hiddenToggle=null;for(Frame frame:Frame.getFrames())if(frame.isShowing()&&frame instanceof JFrame f){W02bGuiProbe.walk(f.getContentPane());if(f.getJMenuBar()!=null)W02bGuiProbe.walk(f.getJMenuBar());}
            });if(W02bGuiProbe.tables.size()==2&&W02bGuiProbe.hiddenToggle!=null&&W02bGuiProbe.names(W02bGuiProbe.tables.get(0)).contains("sample.zip"))break;Thread.sleep(200);
        }
        if(W02bGuiProbe.hiddenToggle==null)throw new IllegalStateException("Original menus were not ready");
        W02bGuiProbe.capture("original-initial");
        Path archive=fixture.resolve("left/sample.zip");navigate(0,archive,"folder","root.txt");W02bGuiProbe.capture("original-zip-root");
        W02bGuiProbe.toggle(true);W02bGuiProbe.capture("original-zip-hidden");
        navigate(0,archive.resolve("folder"),"child.txt","empty");W02bGuiProbe.capture("original-zip-child");
        navigate(0,archive,"folder","root.txt");
        SwingUtilities.invokeAndWait(()->{try{W03GuiProbe.mark(0,"root.txt","folder","root.txt");}catch(Exception e){throw new RuntimeException(e);}});
        W03GuiProbe.copy(0,"SKIP","original-selected-review");awaitFile(fixture.resolve("right/folder/child.txt"),"nested ZIP contents");awaitNames(1,"folder","root.txt");W02bGuiProbe.capture("original-selected-complete");
        if(Files.size(fixture.resolve("right/root.txt"))!=13)throw new IllegalStateException("SKIP overwrote root.txt");
        navigate(0,fixture.resolve("left"),"keep.txt","sample.zip");SwingUtilities.invokeAndWait(()->{try{W03GuiProbe.mark(0,"sample.zip","sample.zip");}catch(Exception e){throw new RuntimeException(e);}});
        unpack(0,"original-whole-review");awaitFile(fixture.resolve("right/root.txt"),"new root ZIP contents");W02bGuiProbe.capture("original-whole-complete");
        navigate(1,fixture.resolve("right/return.zip"),"reply.txt");W02bGuiProbe.capture("original-right-zip");
        SwingUtilities.invokeAndWait(()->{try{W03GuiProbe.mark(1,"reply.txt","reply.txt");}catch(Exception e){throw new RuntimeException(e);}});
        W03GuiProbe.copy(1,"SKIP","original-reverse-review");awaitFile(fixture.resolve("left/reply.txt"),"return ZIP contents");awaitNames(0,"reply.txt");W02bGuiProbe.capture("original-reverse-complete");
        navigate(1,fixture.resolve("right"),"return.zip","root.txt");W02bGuiProbe.capture("original-final");
        System.out.println("W06 original ZIP browsing both panes, selected/full extraction, SKIP/OVERWRITE and three actual UnpackJobs completed");
    }
}
