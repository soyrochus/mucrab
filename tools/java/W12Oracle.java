// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors
import com.google.gson.*;
import com.mucommander.commons.file.*;
import com.mucommander.search.SearchBuilder;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.function.Predicate;
/** Calls the pinned filename predicate. Fields set criteria; no matcher overrides. */
public final class W12Oracle {
 static class Request {String id,root,name,pattern;boolean case_sensitive;}
 @SuppressWarnings("unchecked")
 static Object run(Request r)throws Exception {
  SearchBuilder builder=SearchBuilder.newSearch().what(r.pattern);
  Field sensitive=SearchBuilder.class.getDeclaredField("matchCaseSensitive");sensitive.setAccessible(true);sensitive.setBoolean(builder,r.case_sensitive);
  Method matcher=SearchBuilder.class.getDeclaredMethod("createFilenamePredicate");matcher.setAccessible(true);
  Predicate<AbstractFile> predicate=(Predicate<AbstractFile>)matcher.invoke(builder);
  AbstractFile file=FileFactory.getFile(r.root+"/"+r.name);
  return Map.of("id",r.id,"matches",predicate.test(file),"observed_name",file.getName());
 }
 public static void main(String[] args)throws Exception {
  PrintStream protocol=System.out;System.setOut(System.err);Gson gson=new Gson();
  try(BufferedReader in=new BufferedReader(new InputStreamReader(System.in,StandardCharsets.UTF_8))){String line;while((line=in.readLine())!=null){protocol.println(gson.toJson(run(gson.fromJson(line,Request.class))));protocol.flush();}}
 }
}
