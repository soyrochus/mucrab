// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
import com.google.gson.*;
import com.mucommander.commons.file.*;
import com.mucommander.commons.file.util.FileSet;
import com.mucommander.job.FileJobState;
import com.mucommander.job.impl.*;
import com.mucommander.text.*;
import com.mucommander.core.LocalLocationHistory;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
/** Executes pinned jobs/history/matching; fields only initialize isolated state. */
public final class W10Oracle {
 static class Request {String id, operation, directory, destination, query, name;List<String> paths;int rewind=-1;}
 static Object invoke(Object o,Class<?> c,String n,Class<?>[] t,Object... a)throws Exception{Method m=c.getDeclaredMethod(n,t);m.setAccessible(true);try{return m.invoke(o,a);}catch(InvocationTargetException e){throw new RuntimeException(e.getCause());}}
 static Object run(Request r)throws Exception{
  if(r.operation.equals("quick")){
   Class<?> cls=Class.forName("com.mucommander.ui.main.table.FileTable$FileTableQuickSearch");
   Object q=sun.reflect.ReflectionFactory.getReflectionFactory().newConstructorForSerialization(cls,Object.class.getDeclaredConstructor()).newInstance();
   Class<?> base=Class.forName("com.mucommander.ui.quicksearch.QuickSearch");
   for(String field:List.of("searchString","active")){Field f=base.getDeclaredField(field);f.setAccessible(true);f.set(q,field.equals("active")?Boolean.TRUE:r.query);}
   return Map.of("id",r.id,"matches",base.getMethod("matches",String.class).invoke(q,r.name));
  }
  if(r.operation.equals("history")){
   LocalLocationHistory h=new LocalLocationHistory(null);
   for(String p:r.paths)h.tryToAddToHistory(FileURL.getFileURL(p));
   if(r.rewind>=0){Field f=LocalLocationHistory.class.getDeclaredField("historyIndex");f.setAccessible(true);f.setInt(h,r.rewind);h.tryToAddToHistory(FileURL.getFileURL(r.destination));}
   Field f=LocalLocationHistory.class.getDeclaredField("history");f.setAccessible(true);List<?> all=(List<?>)f.get(h);
   Field i=LocalLocationHistory.class.getDeclaredField("historyIndex");i.setAccessible(true);
   return Map.of("id",r.id,"length",all.size(),"cursor",i.getInt(h),"back",h.hasBackFolder(),"forward",h.hasForwardFolder());
  }
  AbstractFile base=FileFactory.getFile(r.directory);FileSet files=new FileSet(base);
  for(String p:r.paths)files.add(FileFactory.getFile(p));
  FileJob job;
  if(r.operation.equals("mkdir"))job=new MkdirJob(null,null,files);
  else if(r.operation.equals("delete"))job=new DeleteJob(null,null,files,false);
  else job=new CopyJob(null,null,files,FileFactory.getFile(r.destination),null,CopyJob.TransferMode.COPY,com.mucommander.ui.dialog.file.FileCollisionDialog.FileCollisionAction.SKIP);
  job.setAutoSkipErrors(true);invoke(job,FileJob.class,"setState",new Class[]{FileJobState.class},FileJobState.RUNNING);
  List<Boolean> outcomes=new ArrayList<>();
  for(int i=0;i<files.size();i++){
   AbstractFile file=files.get(i);invoke(job,FileJob.class,"nextFile",new Class[]{AbstractFile.class},file);
   outcomes.add((Boolean)invoke(job,job.getClass(),"processFile",new Class[]{AbstractFile.class,Object.class},file,null));
  }
  return Map.of("id",r.id,"outcomes",outcomes);
 }
 public static void main(String[] args)throws Exception{
  PrintStream protocol=System.out;System.setOut(System.err);
  ResourceBundle dictionary=ResourceBundle.getBundle("dictionary",Locale.ENGLISH);Translator.init(dictionary,dictionary,List.of(Locale.US));CustomDateFormat.init();com.mucommander.core.desktop.DesktopManager.init(false);
  Gson gson=new GsonBuilder().serializeNulls().create();try(BufferedReader in=new BufferedReader(new InputStreamReader(System.in,StandardCharsets.UTF_8))){String line;while((line=in.readLine())!=null){Request r=gson.fromJson(line,Request.class);protocol.println(gson.toJson(run(r)));protocol.flush();}}
 }
}
