// SPDX-License-Identifier: GPL-3.0-only
import com.google.gson.*;import com.mucommander.commons.file.*;import com.mucommander.commons.file.protocol.sftp.*;import com.mucommander.commons.file.connection.*;import java.io.*;import java.nio.charset.StandardCharsets;import java.util.*;
/** Calls the pinned SFTP provider against disposable server fixtures, not a rewritten provider. */
public class W15Oracle {
 static class Request {String id,host,user,password,path,operation,local;int port;}
 static Object run(Request r)throws Exception{
  FileURL url=FileURL.getFileURL("sftp://"+r.host+":"+r.port+"/");url.setCredentials(new Credentials(r.user,r.password));url.setPath(r.path);
  AbstractFile file=FileFactory.getFile(url,true);
  if("list".equals(r.operation)){List<Object> entries=new ArrayList<>();for(AbstractFile child:file.ls()){entries.add(Map.of("name",child.getName(),"directory",child.isDirectory(),"link",child.isSymlink(),"size",child.getSize(),"modified_ms",child.getDate()));}return Map.of("id",r.id,"entries",entries);}
  if("upload".equals(r.operation)){try(InputStream in=new FileInputStream(r.local);OutputStream out=file.getOutputStream()){in.transferTo(out);}return Map.of("id",r.id,"ok",true);}
  if("download".equals(r.operation)){try(InputStream in=file.getInputStream();OutputStream out=new FileOutputStream(r.local)){in.transferTo(out);}return Map.of("id",r.id,"ok",true);}
  return Map.of("id",r.id,"error","Unknown test operation");
 }
 public static void main(String[] args)throws Exception{PrintStream result=System.out;System.setOut(System.err);ResourceBundle d=ResourceBundle.getBundle("dictionary",Locale.ENGLISH);com.mucommander.text.Translator.init(d,d,List.of(Locale.US));FileURL.registerHandler("sftp",new DefaultSchemeHandler(new DefaultSchemeParser(),22,"/",AuthenticationType.AUTHENTICATION_REQUIRED,null));FileFactory.registerProtocol("sftp",new SFTPProtocolProvider());Gson json=new Gson();try(BufferedReader input=new BufferedReader(new InputStreamReader(System.in,StandardCharsets.UTF_8))){for(String line;(line=input.readLine())!=null;){Request r=json.fromJson(line,Request.class);try{result.println(json.toJson(run(r)));}catch(Throwable e){e.printStackTrace(System.err);result.println(json.toJson(Map.of("id",r.id,"error",e.getClass().getName())));}result.flush();}}System.exit(0);}
}
