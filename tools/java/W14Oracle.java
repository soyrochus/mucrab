// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
import com.google.gson.*;import com.mucommander.commons.file.*;import java.io.*;import java.lang.reflect.*;import java.nio.charset.StandardCharsets;import java.nio.file.*;import java.util.*;import javax.imageio.ImageIO;import java.awt.image.BufferedImage;
/** Pinned TextViewer.loadDocument with only editor field initialization; JDK ImageIO
 * is the exact loader called by the recovered ImageViewer, a separate boundary. */
public final class W14Oracle {
 static class Request {String id,path,encoding;}
 static Object run(Request r)throws Exception{
  if(r.encoding!=null){Class<?> c=Class.forName("com.mucommander.viewer.text.TextViewer");Object viewer=sun.reflect.ReflectionFactory.getReflectionFactory().newConstructorForSerialization(c,Object.class.getDeclaredConstructor()).newInstance();
   Field f=c.getDeclaredField("textEditorImpl");f.setAccessible(true);Class<?> editorClass=f.getType();Object editor=sun.reflect.ReflectionFactory.getReflectionFactory().newConstructorForSerialization(editorClass,Object.class.getDeclaredConstructor()).newInstance();Field areaField=editorClass.getDeclaredField("textArea");areaField.setAccessible(true);areaField.set(editor,areaField.getType().getDeclaredConstructor().newInstance());f.set(viewer,editor);
   Method m=c.getDeclaredMethod("loadDocument",AbstractFile.class,InputStream.class,String.class,javax.swing.event.DocumentListener.class);m.setAccessible(true);try(InputStream in=Files.newInputStream(Path.of(r.path))){m.invoke(viewer,FileFactory.getFile(r.path),in,r.encoding,null);}Field area=editor.getClass().getDeclaredField("textArea");area.setAccessible(true);String text=((javax.swing.text.JTextComponent)area.get(editor)).getText();return Map.of("id",r.id,"text",text);
  }
  BufferedImage image=ImageIO.read(new File(r.path));if(image==null)return Map.of("id",r.id,"error","No image reader");List<Integer> rgba=new ArrayList<>();for(int y=0;y<image.getHeight();y++)for(int x=0;x<image.getWidth();x++){int argb=image.getRGB(x,y);rgba.add((argb>>16)&255);rgba.add((argb>>8)&255);rgba.add(argb&255);rgba.add((argb>>>24)&255);}return Map.of("id",r.id,"width",image.getWidth(),"height",image.getHeight(),"rgba",rgba);
 }
 public static void main(String[] args)throws Exception{PrintStream p=System.out;System.setOut(System.err);ResourceBundle d=ResourceBundle.getBundle("dictionary",Locale.ENGLISH);com.mucommander.text.Translator.init(d,d,List.of(Locale.US));com.mucommander.text.CustomDateFormat.init();Gson g=new Gson();try(BufferedReader in=new BufferedReader(new InputStreamReader(System.in,StandardCharsets.UTF_8))){String line;while((line=in.readLine())!=null){Request r=g.fromJson(line,Request.class);try{p.println(g.toJson(run(r)));}catch(Throwable e){e.printStackTrace(System.err);p.println(g.toJson(Map.of("id",r.id,"error",e.toString())));}p.flush();}}}
}
