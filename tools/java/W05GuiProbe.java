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

/** Operates actual batch preview/confirmation/collision controls and full jobs. */
public final class W05GuiProbe {
    static final java.util.List<String> previews=new ArrayList<>();
    public static void premain(String output,Instrumentation unused){
        W02bGuiProbe.output=Path.of(output);
        new Thread(()->{try{probe();System.exit(0);}catch(Throwable e){e.printStackTrace();System.exit(2);}},"W05-original-GUI-probe").start();
    }
    static Object field(Object o,String name)throws Exception{return W04GuiProbe.field(o,name);}
    static JDialog await(String simple)throws Exception{
        JDialog[] result={null};
        for(int i=0;i<150;i++){SwingUtilities.invokeAndWait(()->{for(Window w:Window.getWindows())if(w.isShowing()&&w.getClass().getSimpleName().equals(simple))result[0]=(JDialog)w;});if(result[0]!=null)return result[0];Thread.sleep(100);}
        throw new IllegalStateException("Missing original "+simple);
    }
    static void render(JDialog dialog,String label)throws Exception{
        SwingUtilities.invokeAndWait(()->{try{BufferedImage image=new BufferedImage(dialog.getWidth(),dialog.getHeight(),BufferedImage.TYPE_INT_RGB);Graphics2D g=image.createGraphics();dialog.printAll(g);g.dispose();ImageIO.write(image,"png",W02bGuiProbe.output.resolve(label+"-dialog.png").toFile());}catch(Exception e){throw new RuntimeException(e);}});
    }
    static JDialog open(int side)throws Exception{
        SwingUtilities.invokeLater(()->{try{
            JTable table=W02bGuiProbe.tables.get(side);Object frame=field(table,"mainFrame");Method active=frame.getClass().getDeclaredMethod("setActiveTable",table.getClass());active.setAccessible(true);active.invoke(frame,table);
            Object files=W02bGuiProbe.invoke(table,"getSelectedFiles",new Class[]{});Class<?> cls=table.getClass().getClassLoader().loadClass("com.mucommander.ui.dialog.file.BatchRenameDialog");Object dialog=cls.getConstructor(frame.getClass(),files.getClass()).newInstance(frame,files);W02bGuiProbe.invoke(dialog,"showDialog",new Class[]{});
        }catch(Exception e){e.printStackTrace();System.exit(2);}});
        return await("BatchRenameDialog");
    }
    static void text(JDialog d,String field,String text)throws Exception{SwingUtilities.invokeAndWait(()->{try{((JTextField)field(d,field)).setText(text);}catch(Exception e){throw new RuntimeException(e);}});Thread.sleep(150);}
    static void click(JDialog d,String field)throws Exception{SwingUtilities.invokeLater(()->{try{((JButton)field(d,field)).doClick();}catch(Exception e){e.printStackTrace();System.exit(2);}});Thread.sleep(200);}
    static void observe(JDialog d,String label)throws Exception{
        SwingUtilities.invokeAndWait(()->{try{
            JTable table=(JTable)field(d,"tblNames");java.util.List<String> rows=new ArrayList<>();
            for(int i=0;i<table.getRowCount();i++)rows.add("{\"name\":"+W02bGuiProbe.quote((String)table.getValueAt(i,0))+",\"new_name\":"+W02bGuiProbe.quote((String)table.getValueAt(i,1))+"}");
            previews.add("{\"stage\":"+W02bGuiProbe.quote(label)+",\"rename_enabled\":"+((JButton)field(d,"btnRename")).isEnabled()+",\"rows\":["+String.join(",",rows)+"]}");
            Files.writeString(W02bGuiProbe.output.resolve("previews.json"),"["+String.join(",",previews)+"]\n");
        }catch(Exception e){throw new RuntimeException(e);}});render(d,label);W02bGuiProbe.capture(label);
    }
    static JButton named(Component c,String name){if(c instanceof JButton b&&name.equals(b.getText()))return b;if(c instanceof Container ct)for(Component child:ct.getComponents()){JButton b=named(child,name);if(b!=null)return b;}return null;}
    static void awaitComplete(int side)throws Exception{
        for(int i=0;i<150;i++){boolean[] ready={false};SwingUtilities.invokeAndWait(()->{try{JTable t=W02bGuiProbe.tables.get(side);ready[0]=side==0?W02bGuiProbe.names(t).contains("new-two.bin")&&!W02bGuiProbe.names(t).contains("old-two.bin"):W02bGuiProbe.names(t).containsAll(java.util.List.of("item-001.txt","item-002.txt"))&&W04GuiProbe.marked(t).isEmpty();}catch(Exception e){throw new RuntimeException(e);}});if(ready[0]){Thread.sleep(500);return;}Thread.sleep(100);}throw new IllegalStateException("Batch job/refresh did not finish");
    }
    static void probe()throws Exception{
        Files.createDirectories(W02bGuiProbe.output);
        for(int i=0;i<150;i++){SwingUtilities.invokeAndWait(()->{
            for(Window w:Window.getWindows())if(w instanceof Dialog d&&d.isShowing()&&d.getTitle().contains("Welcome"))W02bGuiProbe.dismissWelcome(d);
            W02bGuiProbe.tables.clear();W02bGuiProbe.hiddenToggle=null;for(Frame f:Frame.getFrames())if(f.isShowing()&&f instanceof JFrame jf){W02bGuiProbe.walk(jf.getContentPane());if(jf.getJMenuBar()!=null)W02bGuiProbe.walk(jf.getJMenuBar());}
        });if(W02bGuiProbe.tables.size()==2&&W02bGuiProbe.tables.get(0).getRowCount()>3)break;Thread.sleep(200);}
        W02bGuiProbe.capture("original-initial");
        SwingUtilities.invokeAndWait(()->{try{W04GuiProbe.mark(0,"old-one.txt","package","keep.dat","old-one.txt","old-two.bin");W04GuiProbe.mark(1,"reply.txt","reply.txt");}catch(Exception e){throw new RuntimeException(e);}});
        W02bGuiProbe.capture("original-marked");
        JDialog d=open(0);observe(d,"original-default-preview");text(d,"edtFileNameMask","same");observe(d,"original-duplicate-preview");
        if(((JButton)field(d,"btnRename")).isEnabled())throw new IllegalStateException("Duplicate original preview enabled");
        click(d,"btnClose");W02bGuiProbe.capture("original-cancelled");
        d=open(0);text(d,"edtSearchFor","old-");text(d,"edtReplaceWith","new-");observe(d,"original-forward-preview");click(d,"btnRename");
        JDialog confirm=await("BatchRenameConfirmationDialog");render(confirm,"original-forward-confirmation");W02bGuiProbe.capture("original-forward-confirmation");click(confirm,"btnRename");
        JDialog collision=await("FileCollisionDialog");render(collision,"original-collision-skip");
        SwingUtilities.invokeAndWait(()->{JButton skip=named(collision,"Skip");if(skip==null)throw new IllegalStateException("Missing real Skip button");skip.doClick();});
        awaitComplete(0);W02bGuiProbe.capture("original-forward-complete");
        SwingUtilities.invokeAndWait(()->{try{W04GuiProbe.mark(1,"second.txt","second.txt");}catch(Exception e){throw new RuntimeException(e);}});
        d=open(1);text(d,"edtFileNameMask","item-[C1,1,3].[E]");observe(d,"original-reverse-preview");click(d,"btnRename");confirm=await("BatchRenameConfirmationDialog");render(confirm,"original-reverse-confirmation");click(confirm,"btnRename");awaitComplete(1);W02bGuiProbe.capture("original-reverse-complete");
        System.out.println("W05 actual batch controls, duplicate prevention, cancel, confirmation, Skip collision and both-pane jobs completed");
    }
}
