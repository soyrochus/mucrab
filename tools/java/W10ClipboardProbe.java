// SPDX-License-Identifier: GPL-3.0-only
import com.mucommander.commons.file.*;
import com.mucommander.commons.file.util.FileSet;
import com.mucommander.ui.dnd.ClipboardSupport;
import java.io.*;
/** Native AWT interoperability with the original clipboard implementation. */
public class W10ClipboardProbe {
 public static void main(String[] args)throws Exception{
  PrintStream protocol=System.out;System.setOut(System.err);
  if(args[0].equals("read")){
   FileSet files=ClipboardSupport.getClipboardFiles();if(files==null||files.size()==0)throw new Exception("No original clipboard files");
   for(int i=0;i<files.size();i++)protocol.println(files.get(i).getAbsolutePath());
   protocol.flush();System.exit(0);
  }
  AbstractFile file=FileFactory.getFile(args[1]);FileSet files=new FileSet(file.getParent());files.add(file);ClipboardSupport.setClipboardFiles(files);protocol.println("ready");protocol.flush();new BufferedReader(new InputStreamReader(System.in)).readLine();System.exit(0);
 }
}
