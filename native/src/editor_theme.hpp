#pragma once

#include <QString>

class ScintillaEditBase;
struct StudioTheme;

// Syntax highlighting driven by the active theme. Each Lexilla lexer's style
// numbers are mapped to semantic roles (comment, keyword, type, string…)
// because the numbers mean different things per language.
QString lexerNameForPath(const QString &path);
QString languageNameForPath(const QString &path);
void applyEditorTheme(ScintillaEditBase *editor, const QString &path, const StudioTheme &theme);
// Highlights the bracket under the caret and its match.
void updateBraceHighlight(ScintillaEditBase *editor);
