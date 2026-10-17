// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
import com.google.gson.*;
import com.mucommander.commons.file.*;
import com.mucommander.commons.file.util.FileSet;
import com.mucommander.job.FileJobState;
import com.mucommander.job.impl.*;
import com.mucommander.text.*;
import java.io.*;import java.lang.reflect.*;import java.nio.charset.StandardCharsets;import java.util.*;
/** Invokes the original non-recursive job; records actual files independently. */
public final class W13Oracle {
 static class Request {String id,path;int new_mode;long new_mtime_ms;}
 static Object invoke(Object o,Class<?> c,String n,Class<?>[] t,Object...a)throws Exception{Method m=c.getDeclaredMethod(n,t);m.setAccessible(true);return m.invoke(o,a);}
 static boolean step(ChangeFileAttributesJob job,AbstractFile file)throws Exception{job.setAutoSkipErrors(true);invoke(job,FileJob.class,"setState",new Class[]{FileJobState.class},FileJobState.RUNNING);invoke(job,FileJob.class,"nextFile",new Class[]{AbstractFile.class},file);return (Boolean)invoke(job,ChangeFileAttributesJob.class,"processFile",new Class[]{AbstractFile.class,Object.class},file,null);}
 public static void main(String[] args)throws Exception{
  PrintStream protocol=System.out;System.setOut(System.err);ResourceBundle d=ResourceBundle.getBundle("dictionary",Locale.ENGLISH);Translator.init(d,d,List.of(Locale.US));CustomDateFormat.init();com.mucommander.core.desktop.DesktopManager.init(false);Gson gson=new GsonBuilder().serializeNulls().create();
  try(BufferedReader in=new BufferedReader(new InputStreamReader(System.in,StandardCharsets.UTF_8))){String line;while((line=in.readLine())!=null){Request r=gson.fromJson(line,Request.class);AbstractFile f=FileFactory.getFile(r.path);FileSet files=new FileSet(f.getParent(),f);Map<String,Object> before=new LinkedHashMap<>();before.put("size",f.getSize());before.put("directory",f.isDirectory());before.put("link",f.isSymlink());before.put("permissions",f.getPermissions().getIntValue());before.put("changeable",f.getChangeablePermissions().getIntValue());boolean perms=step(new ChangeFileAttributesJob(null,null,files,r.new_mode,false),f);boolean date=step(new ChangeFileAttributesJob(null,null,files,r.new_mtime_ms,false),f);protocol.println(gson.toJson(Map.of("id",r.id,"before",before,"steps",List.of(perms,date))));protocol.flush();}}
 }
}
