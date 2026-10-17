// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
import com.google.gson.Gson;
import com.mucommander.commons.file.FileURL;
import com.mucommander.ui.main.tabs.*;
import com.mucommander.bookmark.Bookmark;
import com.mucommander.core.LocalLocationHistory;
import java.lang.reflect.Field;
import java.util.*;
/** Supplemental factory/value probe; all invoked behavior belongs to the pinned JAR. */
public final class W11Oracle {
 static int length(LocalLocationHistory history)throws Exception {Field f=LocalLocationHistory.class.getDeclaredField("history");f.setAccessible(true);return ((List<?>)f.get(history)).size();}
 public static void main(String[] args)throws Exception {
  java.io.PrintStream protocol=System.out;System.setOut(System.err);
  FileURL root=FileURL.getFileURL(args[0]),child=FileURL.getFileURL(args[1]);
  FileTableTab origin=new DefaultFileTableTabFactory(null).createTab(root);origin.setLocation(root);origin.setLocation(child);
  FileTableTab copy=new ClonedFileTableTabFactory(null).createTab(origin);int copiedHistory=length(copy.getLocationHistory());boolean sameLocation=origin.getLocation().equals(copy.getLocation());copy.setLocation(root);
  Bookmark original=new Bookmark("Original",args[0]);Bookmark duplicate=(Bookmark)original.clone();duplicate.setName("Renamed");
  protocol.println(new Gson().toJson(Map.of("same_location",sameLocation,"independent_history",length(origin.getLocationHistory())==2,"duplicate_history_length",copiedHistory,"bookmark_clone_independent",original.getName().equals("Original"),"bookmark_same_name_equal",original.equals(new Bookmark("Original",args[1])))));
 }
}
