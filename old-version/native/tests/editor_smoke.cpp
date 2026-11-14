#include <cstdint>
#include <ILexer.h>
#include <ScintillaEditBase.h>
#include <Lexilla.h>
#include <Scintilla.h>
#include <ScintillaMessages.h>

#include <QApplication>

#include <cstring>
#include <string>

int main(int argc, char *argv[])
{
    QApplication application(argc, argv);
    ScintillaEditBase editor;
    editor.send(SCI_SETCODEPAGE, SC_CP_UTF8);

    Scintilla::ILexer5 *lexer = CreateLexer("cpp");
    if (lexer == nullptr) {
        return 1;
    }
    editor.send(SCI_SETILEXER, 0, reinterpret_cast<Scintilla::sptr_t>(lexer));
    bool dirty = false;
    QObject::connect(&editor, &ScintillaEditBase::savePointChanged,
        [&dirty](bool value) { dirty = value; });

    const std::string source = "int main() {\n    return 0;\n}\n";
    editor.send(SCI_SETTEXT, 0, reinterpret_cast<Scintilla::sptr_t>(source.c_str()));
    const auto length = editor.send(SCI_GETLENGTH);
    if (length != static_cast<Scintilla::sptr_t>(source.size())) {
        return 2;
    }

    std::string readback(static_cast<std::size_t>(length) + 1, '\0');
    editor.send(SCI_GETTEXT, static_cast<Scintilla::uptr_t>(readback.size()),
        reinterpret_cast<Scintilla::sptr_t>(readback.data()));
    readback.resize(static_cast<std::size_t>(length));
    if (readback != source) {
        return 3;
    }

    editor.send(SCI_SEARCHANCHOR);
    const auto match = editor.sends(SCI_SEARCHNEXT, SCFIND_NONE, "main");
    if (match < 0) {
        return 4;
    }
    editor.send(SCI_SETSEL, static_cast<Scintilla::uptr_t>(match), match + 4);
    editor.send(SCI_SETSAVEPOINT);
    editor.send(SCI_APPENDTEXT, 1, reinterpret_cast<Scintilla::sptr_t>("!"));
    if (editor.send(SCI_GETLENGTH) != length + 1) {
        return 5;
    }
    if (!dirty) {
        return 6;
    }
    editor.send(SCI_SETSAVEPOINT);
    if (dirty) {
        return 7;
    }
    return 0;
}
