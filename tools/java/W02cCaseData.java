// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 muCrab contributors

/** Snapshot original JDK simple-case data; never used as the comparison oracle. */
public final class W02cCaseData {
    public static void main(String[] args) {
        StringBuilder out = new StringBuilder("{\"java_version\":\"" + System.getProperty("java.version") + "\",\"fold\":[");
        boolean first = true;
        for (int cp = 0; cp <= Character.MAX_CODE_POINT; cp++) {
            if (cp >= 0xd800 && cp <= 0xdfff) continue;
            int folded = Character.toLowerCase(Character.toUpperCase(cp));
            if (folded == cp) continue;
            if (!first) out.append(','); first = false;
            out.append('[').append(cp).append(',').append(folded).append(']');
        }
        out.append("]}"); System.out.println(out);
        System.err.println("BMP private-use vs supplementary: " + "\ue000".compareToIgnoreCase("\ud800\udc00"));
    }
}
