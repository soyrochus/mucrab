// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
import java.awt.*;
import java.lang.instrument.Instrumentation;
import java.lang.reflect.*;
import java.nio.file.*;
import java.util.*;
import javax.swing.*;

/** Observe original start/run/interrupt/pause against a real MainFrame.
 * No bytecode transformation or application method overrides. */
public final class W07GuiProbe {
    static Path output;
    static ClassLoader loader;
    static Object frame;
    static Object currentJob;
    static final java.util.List<String> transitions=Collections.synchronizedList(new ArrayList<>());
    static Object field(Object target,String name)throws Exception {
        for(Class<?> owner=target.getClass();owner!=null;owner=owner.getSuperclass())try{Field f=owner.getDeclaredField(name);f.setAccessible(true);return f.get(target);}catch(NoSuchFieldException ignored){}
        throw new NoSuchFieldException(name);
    }
    static Object call(Object target,String name,Object... args)throws Exception {
        Class<?> type=target instanceof Class<?> c?c:target.getClass();
        for(Method method:type.getMethods()) {
            if(!method.getName().equals(name)||method.getParameterCount()!=args.length)continue;
            Class<?>[] params=method.getParameterTypes();boolean valid=true;
            for(int i=0;i<args.length;i++)if(args[i]!=null&&!params[i].isInstance(args[i])&&!(params[i]==boolean.class&&args[i] instanceof Boolean)&&!(params[i]==long.class&&args[i] instanceof Long)&&!(params[i]==int.class&&args[i] instanceof Integer))valid=false;
            if(valid)return method.invoke(target instanceof Class<?>?null:target,args);
        }
        throw new NoSuchMethodException(type+"."+name);
    }
    static Object file(Path path)throws Exception{return call(loader.loadClass("com.mucommander.commons.file.FileFactory"),"getFile",path.toString());}
    static Object value(String type,String name)throws Exception {for(Object value:loader.loadClass(type).getEnumConstants())if(((Enum<?>)value).name().equals(name))return value;throw new IllegalArgumentException(name);}
    static Object construct(String name,Object... args)throws Exception {
        for(Constructor<?> constructor:loader.loadClass(name).getConstructors()) {
            if(constructor.getParameterCount()!=args.length)continue;
            Class<?>[] params=constructor.getParameterTypes();boolean valid=true;
            for(int i=0;i<args.length;i++)if(args[i]!=null&&!params[i].isInstance(args[i])&&!(params[i]==boolean.class&&args[i] instanceof Boolean))valid=false;
            if(valid)return constructor.newInstance(args);
        }throw new NoSuchMethodException(name);
    }
    public static void premain(String folder,Instrumentation unused) {
        output=Path.of(folder);W02bGuiProbe.output=output;
        new Thread(()->{try{probe();System.exit(0);}catch(Throwable error){error.printStackTrace();System.exit(2);}},"W07-original-observer").start();
    }
    static void ready()throws Exception {
        Files.createDirectories(output);
        for(int i=0;i<150;i++){
            SwingUtilities.invokeAndWait(()->{
                for(Window window:Window.getWindows())if(window instanceof Dialog dialog&&dialog.isShowing()&&dialog.getTitle().contains("Welcome"))W02bGuiProbe.dismissWelcome(dialog);
                W02bGuiProbe.tables.clear();for(Window window:Window.getWindows())if(window.isShowing())W02bGuiProbe.walk(window);
            });
            if(W02bGuiProbe.tables.size()==2)break;Thread.sleep(100);
        }
        if(W02bGuiProbe.tables.size()!=2)throw new IllegalStateException("No original tables");
        loader=W02bGuiProbe.tables.get(0).getClass().getClassLoader();frame=field(W02bGuiProbe.tables.get(0),"mainFrame");
    }
    static void navigate(int side,Path path)throws Exception {
        Object[] worker={null};
        for(int i=0;i<100&&worker[0]==null;i++) {
            SwingUtilities.invokeAndWait(()->{try{worker[0]=call(field(W02bGuiProbe.tables.get(side),"folderPanel"),"tryChangeCurrentFolder",file(path));}catch(Exception e){throw new RuntimeException(e);}});
            if(worker[0]==null)Thread.sleep(50);
        }
        if(worker[0] instanceof Thread thread)thread.join(10000);
        for(int i=0;i<100;i++){
            boolean[] ready={false};SwingUtilities.invokeAndWait(()->{try{Object folder=call(field(W02bGuiProbe.tables.get(side),"folderPanel"),"getCurrentFolder");ready[0]=Path.of((String)call(folder,"getAbsolutePath")).normalize().equals(path.normalize());}catch(Exception e){throw new RuntimeException(e);}});
            if(ready[0])return;Thread.sleep(50);
        }throw new IllegalStateException("Navigation failed: "+path);
    }
    static void run(String[] row)throws Exception {
        String id=row[0],action=row[1],mode=row[2];Path source=Path.of(row[3]),destination=Path.of(row[4]);
        String[] names=row[5].split(",");long cancelBytes=Long.parseLong(row[6]);
        navigate(0,source);navigate(1,destination);
        for(int i=0;i<100;i++) {
            boolean[] ready={false}; SwingUtilities.invokeAndWait(()->{try{ready[0]=W02bGuiProbe.names(W02bGuiProbe.tables.get(0)).containsAll(Arrays.asList(names));}catch(Exception e){throw new RuntimeException(e);}});
            if(ready[0])break;Thread.sleep(50);
            if(i==99)throw new IllegalStateException("Original listing not ready for marking");
        }
        SwingUtilities.invokeAndWait(()->{try{
            Method active=frame.getClass().getDeclaredMethod("setActiveTable",W02bGuiProbe.tables.get(0).getClass());active.setAccessible(true);active.invoke(frame,W02bGuiProbe.tables.get(0));
            java.util.List<String> visible=W02bGuiProbe.names(W02bGuiProbe.tables.get(0));
            for(String name:names)call(W02bGuiProbe.tables.get(0),"setRowMarked",visible.indexOf(name),true);
        }catch(Exception e){throw new RuntimeException(e);}});
        Object files=construct("com.mucommander.commons.file.util.FileSet",file(source));
        for(String name:names)call(files,"add",file(source.resolve(name)));
        Object policy=value("com.mucommander.ui.dialog.file.FileCollisionDialog$FileCollisionAction",mode.equals("skip")?"SKIP":"OVERWRITE");
        currentJob=switch(action) {
            case "copy" -> construct("com.mucommander.job.impl.CopyJob",null,frame,files,file(destination),null,value("com.mucommander.job.impl.CopyJob$TransferMode","COPY"),policy);
            case "move","rename" -> construct("com.mucommander.job.impl.MoveJob",null,frame,files,file(destination),action.equals("rename")?"renamed.bin":null,policy,action.equals("rename"));
            case "batch" -> construct("com.mucommander.job.impl.BatchRenameJob",null,frame,files,Arrays.stream(names).map(name->"new_"+name).toList());
            case "extract" -> construct("com.mucommander.job.impl.UnpackJob",null,frame,files,file(destination),policy);
            default -> throw new IllegalArgumentException(action);
        };
        call(currentJob,"setAutoSkipErrors",true);call(currentJob,"setIntegrityCheckEnabled",false);
        if(mode.equals("stream")||mode.equals("nested")||mode.equals("pause"))call(currentJob,"setThroughputLimit",262144L);
        transitions.clear();transitions.add("NOT_STARTED");
        Class<?> listenerType=loader.loadClass("com.mucommander.job.FileJobListener");
        Object listener=Proxy.newProxyInstance(loader,new Class<?>[]{listenerType},(proxy,method,args)->{
            if(method.getName().equals("hashCode"))return System.identityHashCode(proxy);
            if(method.getName().equals("equals"))return proxy==args[0];
            if(method.getName().equals("toString"))return "W07 observer";
            if(method.getName().equals("jobStateChanged")){
                String state=args[2].toString();transitions.add(state);
                if(mode.equals("before")&&state.equals("RUNNING"))call(currentJob,"interrupt");
            }return null;
        });
        call(currentJob,"addFileJobListener",listener);
        Object startedJob=currentJob;
        SwingUtilities.invokeAndWait(()->{try{call(startedJob,"start");}catch(Exception e){throw new RuntimeException(e);}});
        boolean paused=false,pauseStable=true;long deadline=System.currentTimeMillis()+20000;
        while(System.currentTimeMillis()<deadline){
            String state=call(currentJob,"getState").toString();
            if(state.equals("FINISHED")||state.equals("INTERRUPTED"))break;
            long bytes=(Long)call(currentJob,"getCurrentFileByteCount");
            Object currentFile=call(currentJob,"getCurrentFile");
            String path=currentFile==null?"":(String)call(currentFile,"getName");
            if(path.equals("large.bin")||action.equals("extract")){
                if(mode.equals("pause")&&!paused&&bytes>=65536){
                    call(currentJob,"setPaused",true);Thread.sleep(400);
                    long before=(Long)call(currentJob,"getCurrentFileByteCount");Thread.sleep(200);
                    pauseStable=before==(Long)call(currentJob,"getCurrentFileByteCount");
                    call(currentJob,"setPaused",false);paused=true;
                }
                if((mode.equals("stream")||mode.equals("nested")||mode.equals("pause"))&&bytes>=cancelBytes)call(currentJob,"interrupt");
            }
            Thread.sleep(2);
        }
        // INTERRUPTED is only a request; wait for the actual original job thread
        // to leave run(), including output close, table refresh and jobEnded.
        for(int i=0;i<200;i++){
            boolean alive=Thread.getAllStackTraces().keySet().stream().anyMatch(t->t.isAlive()&&t.getName().equals(startedJob.getClass().getName()));
            if(!alive)break;Thread.sleep(20);
            if(i==199)throw new IllegalStateException("Original worker did not acknowledge stop");
        }
        String state=call(currentJob,"getState").toString();
        if(!state.equals("FINISHED")&&!state.equals("INTERRUPTED"))throw new IllegalStateException("Job did not terminate: "+id+" "+state);
        call(currentJob,"interrupt"); // terminal interrupt is idempotent
        Thread.sleep(150);
        java.util.List<String> marked=new ArrayList<>();
        SwingUtilities.invokeAndWait(()->{try{
            JTable table=W02bGuiProbe.tables.get(0);java.util.List<String> visible=W02bGuiProbe.names(table);
            for(int i=0;i<visible.size();i++)if((Boolean)call(table.getModel(),"isRowMarked",i))marked.add(visible.get(i));
        }catch(Exception e){throw new RuntimeException(e);}});
        String states=String.join(",",transitions);
        String json="{\"id\":"+W02bGuiProbe.quote(id)+",\"state\":"+W02bGuiProbe.quote(state)+",\"transitions\":"+W02bGuiProbe.quote(states)+",\"pause_stable\":"+pauseStable+",\"marked\":["+String.join(",",marked.stream().map(W02bGuiProbe::quote).toList())+"]}";
        Files.writeString(output.resolve(id+".json"),json+"\n");
        System.out.println("W07 "+json);
        if(mode.equals("pause"))SwingUtilities.invokeAndWait(()->{try {
            Component window=SwingUtilities.getWindowAncestor(W02bGuiProbe.tables.get(0));
            java.awt.image.BufferedImage image=new java.awt.image.BufferedImage(window.getWidth(),window.getHeight(),java.awt.image.BufferedImage.TYPE_INT_RGB);
            Graphics2D graphics=image.createGraphics();window.printAll(graphics);graphics.dispose();
            javax.imageio.ImageIO.write(image,"png",output.resolve("original-pause-cancel.png").toFile());
        }catch(Exception e){throw new RuntimeException(e);}});
    }
    static void probe()throws Exception {
        ready();
        for(String row:Files.readAllLines(output.getParent().resolve("original-requests.tsv")))run(row.split("\t",-1));
    }
}
