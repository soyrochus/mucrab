// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
import com.google.gson.*;
import com.mucommander.commons.file.*;
import com.mucommander.commons.file.util.FileSet;
import com.mucommander.job.FileJobState;
import com.mucommander.job.impl.*;
import com.mucommander.text.*;
import com.mucommander.ui.dialog.file.BatchRenameDialog;
import com.mucommander.ui.dialog.file.FileCollisionDialog.FileCollisionAction;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import javax.swing.*;
import sun.reflect.ReflectionFactory;

/** Field-only headless wiring; all pattern, preview and job methods are original. */
public final class W05Oracle {
    static final class Options {
        String mask="[N].[E]", search="", replacement="", counter_start="1", counter_step="1";
        int case_mode=0, digits=1;
    }
    static final class Request { String id,directory; List<String> names; List<Options> options; boolean apply; }
    static void set(Object target,Class<?> owner,String name,Object value) throws Exception {
        Field f=owner.getDeclaredField(name);f.setAccessible(true);f.set(target,value);
    }
    static Object get(Object target,String name) throws Exception {
        Field f=BatchRenameDialog.class.getDeclaredField(name);f.setAccessible(true);return f.get(target);
    }
    static Object call(Object target,Class<?> owner,String name,Class<?>[] types,Object...args) throws Exception {
        Method m=owner.getDeclaredMethod(name,types);m.setAccessible(true);
        try{return m.invoke(target,args);}catch(InvocationTargetException e){throw new RuntimeException(e.getCause());}
    }
    static void options(BatchRenameDialog dialog,Options o) throws Exception {
        for(String[] f:new String[][]{{"edtFileNameMask",o.mask},{"edtSearchFor",o.search},{"edtReplaceWith",o.replacement},{"edtCounterStart",o.counter_start},{"edtCounterStep",o.counter_step}}) ((JTextField)get(dialog,f[0])).setText(f[1]);
        ((JComboBox<?>)get(dialog,"cbCase")).setSelectedIndex(o.case_mode);
        ((JComboBox<?>)get(dialog,"cbCounterDigits")).setSelectedIndex(o.digits-1);
        call(dialog,BatchRenameDialog.class,"generateNewNames",new Class[]{});
    }
    static Map<String,Object> preview(BatchRenameDialog dialog,FileSet files) throws Exception {
        List<?> names=(List<?>)get(dialog,"newNames");List<Object> rows=new ArrayList<>();int changed=0;
        for(int i=0;i<files.size();i++){String old=files.get(i).getName();rows.add(Map.of("name",old,"new_name",names.get(i)));if(!old.equals(names.get(i)))changed++;}
        JLabel label=(JLabel)get(dialog,"lblDuplicates");boolean visible=label.isVisible();
        return Map.of("rows",rows,"duplicate_names",visible&&label.getText().equals(Translator.get("batch_rename_dialog.duplicate_names")),
            "old_names_conflict",visible&&label.getText().equals(Translator.get("batch_rename_dialog.names_conflict")),"rename_enabled",((JButton)get(dialog,"btnRename")).isEnabled(),"changed_count",changed);
    }
    static Object run(Request request) throws Exception {
        AbstractFile folder=FileFactory.getFile(request.directory);FileSet files=new FileSet(folder);
        Map<String,AbstractFile> oldNames=new HashMap<>();
        for(String name:request.names){AbstractFile file=folder.getDirectChild(name);files.add(file);oldNames.put(com.mucommander.commons.file.util.PathUtils.removeTrailingSeparator(file.getAbsolutePath()),file);}
        BatchRenameDialog dialog=(BatchRenameDialog)ReflectionFactory.getReflectionFactory().newConstructorForSerialization(BatchRenameDialog.class,Object.class.getDeclaredConstructor()).newInstance();
        Class<?> type=BatchRenameDialog.class;
        set(dialog,type,"files",files);set(dialog,type,"oldNames",oldNames);set(dialog,type,"tokens",new ArrayList<>());
        set(dialog,type,"newNames",new ArrayList<>(Collections.nCopies(files.size(),"")));set(dialog,type,"blockNames",new ArrayList<>(Collections.nCopies(files.size(),false)));
        for(String name:List.of("edtFileNameMask","edtSearchFor","edtReplaceWith","edtCounterStart","edtCounterStep"))set(dialog,type,name,new JTextField());
        set(dialog,type,"cbCase",new JComboBox<>(new String[]{"0","1","2","3","4"}));set(dialog,type,"cbCounterDigits",new JComboBox<>(new String[]{"1","2","3","4","5"}));
        set(dialog,type,"btnRename",new JButton());set(dialog,type,"lblDuplicates",new JLabel());call(dialog,type,"getTblNames",new Class[]{});
        List<Object> previews=new ArrayList<>();options(dialog,new Options());previews.add(preview(dialog,files));
        for(Options opts:request.options){options(dialog,opts);previews.add(preview(dialog,files));}
        List<?> newNames=(List<?>)get(dialog,"newNames");List<String> unchanged=new ArrayList<>();
        for(int i=0;i<files.size();i++)if(files.get(i).getName().equals(newNames.get(i)))unchanged.add(files.get(i).getName());
        boolean applied=request.apply&&((JButton)get(dialog,"btnRename")).isEnabled()&&unchanged.size()<files.size();
        List<Object> outcomes=new ArrayList<>();
        if(applied){
            call(dialog,type,"removeUnchangedFiles",new Class[]{boolean.class},false);
            @SuppressWarnings("unchecked") List<String> changed=(List<String>)get(dialog,"newNames");
            BatchRenameJob job=new BatchRenameJob(null,null,files,changed);job.setAutoSkipErrors(true);job.setIntegrityCheckEnabled(false);
            // Fixed original apply-to-all SKIP choice; interactive ASK is observed separately in the GUI.
            set(job,AbstractCopyJob.class,"defaultFileExistsAction",FileCollisionAction.SKIP);
            call(job,FileJob.class,"setState",new Class[]{FileJobState.class},FileJobState.RUNNING);
            for(int i=0;i<files.size();i++){
                set(job,FileJob.class,"currentFileIndex",i);AbstractFile file=files.get(i);
                call(job,TransferFileJob.class,"nextFile",new Class[]{AbstractFile.class},file);
                boolean success=(Boolean)call(job,BatchRenameJob.class,"processFile",new Class[]{AbstractFile.class,Object.class},file,null);
                outcomes.add(Map.of("name",file.getName(),"new_name",changed.get(i),"success",success));
            }
        }
        return Map.of("id",request.id,"status","ok","previews",previews,"applied",applied,"unchanged_names",unchanged,"outcomes",outcomes);
    }
    public static void main(String[] args) throws Exception {
        PrintStream protocol=System.out;System.setOut(System.err);Locale.setDefault(Locale.US);TimeZone.setDefault(TimeZone.getTimeZone("UTC"));
        ResourceBundle dictionary=ResourceBundle.getBundle("dictionary",Locale.ENGLISH);Translator.init(dictionary,dictionary,List.of(Locale.US));CustomDateFormat.init();
        com.mucommander.core.desktop.DesktopManager.init(false);
        Gson gson=new GsonBuilder().serializeNulls().create();
        try(BufferedReader input=new BufferedReader(new InputStreamReader(System.in,StandardCharsets.UTF_8))){String line;while((line=input.readLine())!=null){Request r=gson.fromJson(line,Request.class);System.err.println("W05 start: "+r.id);protocol.println(gson.toJson(run(r)));protocol.flush();System.err.println("W05 complete: "+r.id);}}
    }
}
