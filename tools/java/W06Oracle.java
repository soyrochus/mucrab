// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
import com.google.gson.*;
import com.mucommander.commons.file.*;
import com.mucommander.commons.file.archive.*;
import com.mucommander.commons.file.archive.zip.*;
import com.mucommander.job.FileJobState;
import com.mucommander.job.impl.*;
import com.mucommander.text.*;
import com.mucommander.ui.dialog.file.FileCollisionDialog.FileCollisionAction;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.util.*;

/** Calls the unchanged ZIP provider, archive tree and UnpackJob methods. */
public final class W06Oracle {
    static final class Request {
        String id, archive, destination, prefix, policy;
        List<String> list_paths, selected;
        boolean extract;
    }
    static Object invoke(Object target, Class<?> owner, String name, Class<?>[] types, Object... args) throws Exception {
        Method method=owner.getDeclaredMethod(name,types);method.setAccessible(true);
        try{return method.invoke(target,args);}catch(InvocationTargetException e){throw new RuntimeException(e.getCause());}
    }
    static Object listing(AbstractFile folder) throws IOException {
        List<Object> entries=new ArrayList<>();
        for(AbstractFile file:folder.ls()){
            Map<String,Object> metadata=new LinkedHashMap<>();
            metadata.put("size",file.getSize());metadata.put("modified_ms",file.getDate());metadata.put("extension",file.getExtension());
            entries.add(Map.of("name",file.getName(),"is_directory",file.isDirectory(),"is_symlink",file.isSymlink(),"metadata",metadata));
        }
        return entries;
    }
    static Object run(Request r) throws Exception {
        AbstractFile file=FileFactory.getFile(r.archive);
        AbstractArchiveFile archive=file.getAncestor(AbstractArchiveFile.class);
        if(archive==null)throw new IllegalStateException("ZIP provider was not used");
        try{archive.ls();}catch(IOException e){return Map.of("id",r.id,"status","error","error","archive_unavailable");}
        List<Object> listings=new ArrayList<>();
        for(String path:r.list_paths){
            try{listings.add(Map.of("path",path,"status","ok","entries",listing(path.isEmpty()?archive:archive.getArchiveEntryFile(path))));}
            catch(IOException e){listings.add(Map.of("path",path,"status","error","entries",List.of()));}
        }
        boolean success=false;
        if(r.extract){
            List<ArchiveEntry> selected=null;
            if(r.selected!=null||!r.prefix.isEmpty()){
                selected=new ArrayList<>();
                AbstractFile folder=r.prefix.isEmpty()?archive:archive.getArchiveEntryFile(r.prefix);
                if(r.selected==null){
                    for(AbstractFile child:folder.ls())selected.add(child.getAncestor(AbstractArchiveEntryFile.class).getEntry());
                }else for(String name:r.selected)selected.add(archive.getArchiveEntryFile(r.prefix+name).getAncestor(AbstractArchiveEntryFile.class).getEntry());
            }
            AbstractFile dest=FileFactory.getFile(r.destination);
            UnpackJob job=new UnpackJob(null,null,archive,r.prefix.isEmpty()?0:ArchiveEntry.getDepth(r.prefix),dest,null,FileCollisionAction.valueOf(r.policy),selected);
            job.setAutoSkipErrors(true);job.setIntegrityCheckEnabled(false);
            invoke(job,FileJob.class,"setState",new Class[]{FileJobState.class},FileJobState.RUNNING);
            invoke(job,TransferFileJob.class,"nextFile",new Class[]{AbstractFile.class},archive);
            success=(Boolean)invoke(job,UnpackJob.class,"processFile",new Class[]{AbstractFile.class,Object.class},archive,null);
        }
        return Map.of("id",r.id,"status","ok","listings",listings,"extracted",r.extract,"success",success);
    }
    public static void main(String[] args)throws Exception{
        PrintStream protocol=System.out;System.setOut(System.err);Locale.setDefault(Locale.US);TimeZone.setDefault(TimeZone.getTimeZone("UTC"));
        ResourceBundle dictionary=ResourceBundle.getBundle("dictionary",Locale.ENGLISH);Translator.init(dictionary,dictionary,List.of(Locale.US));CustomDateFormat.init();
        com.mucommander.core.desktop.DesktopManager.init(false);FileFactory.registerArchiveFormat(new ZipFormatProvider());
        Gson gson=new GsonBuilder().serializeNulls().create();
        try(BufferedReader input=new BufferedReader(new InputStreamReader(System.in,StandardCharsets.UTF_8))){String line;while((line=input.readLine())!=null){Request r=gson.fromJson(line,Request.class);System.err.println("W06 start: "+r.id);protocol.println(gson.toJson(run(r)));protocol.flush();System.err.println("W06 complete: "+r.id);}}
    }
}
