---
name: pdf-documents-discipline
description: PDF read/generate on PC via poppler/reportlab; on phone prefer Termux tools if installed.
---

## Purpose

Handle PDF tasks without corrupting layout or guessing page content.

## PC (preferred for layout)

- Render pages for visual check (Poppler).
- Extract with `pdfplumber` / `pypdf`; generate with `reportlab`.

## Phone / Termux

- If `pdftotext` or python PDF libs exist in Termux, use `exec_shell` and capture output.
- Otherwise: ask user to share text export or process on PC.

## Rules

- Do not claim PDF contents without extraction or render evidence.
- Large PDFs: sample pages + note total page count.
