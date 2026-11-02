#include "editor_theme.hpp"
#include "studio_theme.hpp"

#include <ScintillaEditBase.h>
#include <ILexer.h>
#include <Lexilla.h>
#include <SciLexer.h>
#include <Scintilla.h>

#include <QFileInfo>
#include <QFontDatabase>
#include <QFontInfo>
#include <QHash>
#include <QStringList>

#include <vector>

namespace {

enum class Role { Comment, Keyword, Type, String, Number, Function, Constant, Operator, Preprocessor, Tag,
    Attribute, Variable, Error, Heading };

struct Language {
    const char *lexer;
    const char *name;
    std::vector<std::pair<int, Role>> roles;
    std::vector<const char *> keywords;
};

const char *const CppKeywords = "alignas alignof and asm auto bitand bitor break case catch class co_await co_return "
    "co_yield compl concept const consteval constexpr constinit const_cast continue decltype default delete do "
    "dynamic_cast else enum explicit export extern false final for friend goto if inline mutable namespace new "
    "noexcept not nullptr operator or override private protected public register reinterpret_cast requires return "
    "sizeof static static_assert static_cast struct switch template this thread_local throw true try typedef typeid "
    "typename union using virtual volatile while xor emit signals slots";
const char *const CppTypes = "bool char char8_t char16_t char32_t double float int long short signed unsigned void "
    "wchar_t size_t ssize_t ptrdiff_t int8_t int16_t int32_t int64_t uint8_t uint16_t uint32_t uint64_t qint64 "
    "quint64 qreal qsizetype std string string_view vector map unordered_map set optional variant unique_ptr "
    "shared_ptr QString QStringList QList QHash QMap QByteArray QVariant QObject QWidget QJsonObject QJsonArray";
const char *const JsKeywords = "abstract as async await break case catch class const continue debugger declare "
    "default delete do else enum export extends false finally for from function get if implements import in "
    "instanceof interface is keyof let namespace new null of package private protected public readonly return "
    "satisfies set static super switch this throw true try type typeof undefined var void while with yield";
const char *const JsTypes = "any bigint boolean never number object string symbol unknown Array Promise Record "
    "Partial Map Set Date Error JSON Math console window document React";
const char *const JavaKeywords = "abstract assert break case catch class const continue default do else enum extends "
    "final finally for goto if implements import instanceof interface native new package private protected public "
    "return static strictfp super switch synchronized this throw throws transient try volatile while var record "
    "sealed permits yield true false null as base checked delegate event explicit extern fixed foreach implicit in "
    "internal is lock namespace operator out override params readonly ref sizeof stackalloc struct typeof unchecked "
    "unsafe using virtual async await init get set fun val when object companion data suspend open lateinit";
const char *const JavaTypes = "boolean byte char double float int long short void bool decimal object sbyte string "
    "uint ulong ushort String Integer List Map Set Optional Task Unit Any Int Long Boolean";
const char *const GoKeywords = "break case chan const continue default defer else fallthrough for func go goto if "
    "import interface map package range return select struct switch type var true false nil iota";
const char *const GoTypes = "bool byte complex64 complex128 error float32 float64 int int8 int16 int32 int64 rune "
    "string uint uint8 uint16 uint32 uint64 uintptr any";

const std::vector<std::pair<int, Role>> &cppRoles()
{
    static const std::vector<std::pair<int, Role>> roles = {
        {SCE_C_COMMENT, Role::Comment}, {SCE_C_COMMENTLINE, Role::Comment}, {SCE_C_COMMENTDOC, Role::Comment},
        {SCE_C_COMMENTLINEDOC, Role::Comment}, {SCE_C_COMMENTDOCKEYWORD, Role::Comment},
        {SCE_C_COMMENTDOCKEYWORDERROR, Role::Comment}, {SCE_C_PREPROCESSORCOMMENT, Role::Comment},
        {SCE_C_PREPROCESSORCOMMENTDOC, Role::Comment}, {SCE_C_NUMBER, Role::Number}, {SCE_C_WORD, Role::Keyword},
        {SCE_C_WORD2, Role::Type}, {SCE_C_GLOBALCLASS, Role::Type}, {SCE_C_STRING, Role::String},
        {SCE_C_CHARACTER, Role::String}, {SCE_C_VERBATIM, Role::String}, {SCE_C_STRINGRAW, Role::String},
        {SCE_C_TRIPLEVERBATIM, Role::String}, {SCE_C_HASHQUOTEDSTRING, Role::String}, {SCE_C_STRINGEOL, Role::String},
        {SCE_C_USERLITERAL, Role::Number}, {SCE_C_REGEX, Role::Constant}, {SCE_C_UUID, Role::Constant},
        {SCE_C_ESCAPESEQUENCE, Role::Constant}, {SCE_C_PREPROCESSOR, Role::Preprocessor},
        {SCE_C_OPERATOR, Role::Operator}, {SCE_C_IDENTIFIER, Role::Variable}, {SCE_C_TASKMARKER, Role::Keyword},
    };
    return roles;
}

const QHash<QString, Language> &languages()
{
    static const QHash<QString, Language> table = {
        {QStringLiteral("cpp"), {"cpp", "C++", cppRoles(), {CppKeywords, CppTypes}}},
        {QStringLiteral("c"), {"cpp", "C", cppRoles(), {CppKeywords, CppTypes}}},
        {QStringLiteral("javascript"), {"cpp", "JavaScript", cppRoles(), {JsKeywords, JsTypes}}},
        {QStringLiteral("typescript"), {"cpp", "TypeScript", cppRoles(), {JsKeywords, JsTypes}}},
        {QStringLiteral("java"), {"cpp", "Java", cppRoles(), {JavaKeywords, JavaTypes}}},
        {QStringLiteral("csharp"), {"cpp", "C#", cppRoles(), {JavaKeywords, JavaTypes}}},
        {QStringLiteral("kotlin"), {"cpp", "Kotlin", cppRoles(), {JavaKeywords, JavaTypes}}},
        {QStringLiteral("go"), {"cpp", "Go", cppRoles(), {GoKeywords, GoTypes}}},
        {QStringLiteral("python"), {"python", "Python", {
            {SCE_P_COMMENTLINE, Role::Comment}, {SCE_P_COMMENTBLOCK, Role::Comment}, {SCE_P_NUMBER, Role::Number},
            {SCE_P_STRING, Role::String}, {SCE_P_CHARACTER, Role::String}, {SCE_P_TRIPLE, Role::String},
            {SCE_P_TRIPLEDOUBLE, Role::String}, {SCE_P_FSTRING, Role::String}, {SCE_P_FCHARACTER, Role::String},
            {SCE_P_FTRIPLE, Role::String}, {SCE_P_FTRIPLEDOUBLE, Role::String}, {SCE_P_STRINGEOL, Role::String},
            {SCE_P_WORD, Role::Keyword}, {SCE_P_WORD2, Role::Function}, {SCE_P_CLASSNAME, Role::Type},
            {SCE_P_DEFNAME, Role::Function}, {SCE_P_OPERATOR, Role::Operator}, {SCE_P_IDENTIFIER, Role::Variable},
            {SCE_P_DECORATOR, Role::Preprocessor}, {SCE_P_ATTRIBUTE, Role::Variable},
        }, {"False None True and as assert async await break class continue def del elif else except finally for "
            "from global if import in is lambda match case nonlocal not or pass raise return try while with yield",
            "self cls abs all any bool bytes callable chr classmethod dict dir divmod enumerate eval exec filter float "
            "format frozenset getattr globals hasattr hash hex id input int isinstance issubclass iter len list locals "
            "map max min next object open ord pow print property range repr reversed round set setattr slice sorted "
            "staticmethod str sum super tuple type vars zip Exception ValueError TypeError KeyError OSError RuntimeError"}}},
        {QStringLiteral("rust"), {"rust", "Rust", {
            {SCE_RUST_COMMENTBLOCK, Role::Comment}, {SCE_RUST_COMMENTLINE, Role::Comment},
            {SCE_RUST_COMMENTBLOCKDOC, Role::Comment}, {SCE_RUST_COMMENTLINEDOC, Role::Comment},
            {SCE_RUST_NUMBER, Role::Number}, {SCE_RUST_WORD, Role::Keyword}, {SCE_RUST_WORD2, Role::Type},
            {SCE_RUST_STRING, Role::String}, {SCE_RUST_STRINGR, Role::String}, {SCE_RUST_CHARACTER, Role::String},
            {SCE_RUST_BYTESTRING, Role::String}, {SCE_RUST_OPERATOR, Role::Operator},
            {SCE_RUST_IDENTIFIER, Role::Variable}, {SCE_RUST_LIFETIME, Role::Constant}, {SCE_RUST_MACRO, Role::Function},
            {SCE_RUST_LEXERROR, Role::Error},
        }, {"as async await break const continue crate dyn else enum extern false fn for if impl in let loop match mod "
            "move mut pub ref return self Self static struct super trait true type unsafe use where while",
            "bool char f32 f64 i8 i16 i32 i64 i128 isize str u8 u16 u32 u64 u128 usize String Vec Option Result Box "
            "Some None Ok Err"}}},
        {QStringLiteral("bash"), {"bash", "Shell", {
            {SCE_SH_COMMENTLINE, Role::Comment}, {SCE_SH_NUMBER, Role::Number}, {SCE_SH_WORD, Role::Keyword},
            {SCE_SH_STRING, Role::String}, {SCE_SH_CHARACTER, Role::String}, {SCE_SH_BACKTICKS, Role::String},
            {SCE_SH_HERE_Q, Role::String}, {SCE_SH_OPERATOR, Role::Operator}, {SCE_SH_IDENTIFIER, Role::Variable},
            {SCE_SH_SCALAR, Role::Constant}, {SCE_SH_PARAM, Role::Constant}, {SCE_SH_HERE_DELIM, Role::Preprocessor},
            {SCE_SH_ERROR, Role::Error},
        }, {"if then else elif fi case esac for while until do done in function return exit local export readonly "
            "declare set unset shift break continue source alias echo printf read cd test true false"}}},
        {QStringLiteral("powershell"), {"powershell", "PowerShell", {
            {SCE_POWERSHELL_COMMENT, Role::Comment}, {SCE_POWERSHELL_COMMENTSTREAM, Role::Comment},
            {SCE_POWERSHELL_STRING, Role::String}, {SCE_POWERSHELL_CHARACTER, Role::String},
            {SCE_POWERSHELL_HERE_STRING, Role::String}, {SCE_POWERSHELL_NUMBER, Role::Number},
            {SCE_POWERSHELL_VARIABLE, Role::Constant}, {SCE_POWERSHELL_OPERATOR, Role::Operator},
            {SCE_POWERSHELL_IDENTIFIER, Role::Variable}, {SCE_POWERSHELL_KEYWORD, Role::Keyword},
            {SCE_POWERSHELL_CMDLET, Role::Function}, {SCE_POWERSHELL_ALIAS, Role::Function},
            {SCE_POWERSHELL_FUNCTION, Role::Function},
        }, {"begin break catch class continue data do dynamicparam else elseif end exit filter finally for foreach "
            "from function if in param process return switch throw trap try until using var while"}}},
        {QStringLiteral("json"), {"json", "JSON", {
            {SCE_JSON_NUMBER, Role::Number}, {SCE_JSON_STRING, Role::String}, {SCE_JSON_STRINGEOL, Role::String},
            {SCE_JSON_PROPERTYNAME, Role::Attribute}, {SCE_JSON_ESCAPESEQUENCE, Role::Constant},
            {SCE_JSON_LINECOMMENT, Role::Comment}, {SCE_JSON_BLOCKCOMMENT, Role::Comment},
            {SCE_JSON_OPERATOR, Role::Operator}, {SCE_JSON_URI, Role::Function}, {SCE_JSON_KEYWORD, Role::Keyword},
            {SCE_JSON_LDKEYWORD, Role::Keyword}, {SCE_JSON_ERROR, Role::Error},
        }, {"true false null"}}},
        {QStringLiteral("yaml"), {"yaml", "YAML", {
            {SCE_YAML_COMMENT, Role::Comment}, {SCE_YAML_IDENTIFIER, Role::Attribute}, {SCE_YAML_KEYWORD, Role::Keyword},
            {SCE_YAML_NUMBER, Role::Number}, {SCE_YAML_REFERENCE, Role::Constant}, {SCE_YAML_DOCUMENT, Role::Preprocessor},
            {SCE_YAML_TEXT, Role::String}, {SCE_YAML_ERROR, Role::Error}, {SCE_YAML_OPERATOR, Role::Operator},
        }, {"true false yes no null on off"}}},
        {QStringLiteral("toml"), {"toml", "TOML", {
            {SCE_TOML_COMMENT, Role::Comment}, {SCE_TOML_IDENTIFIER, Role::Variable}, {SCE_TOML_KEYWORD, Role::Keyword},
            {SCE_TOML_NUMBER, Role::Number}, {SCE_TOML_TABLE, Role::Type}, {SCE_TOML_KEY, Role::Attribute},
            {SCE_TOML_ERROR, Role::Error}, {SCE_TOML_OPERATOR, Role::Operator}, {SCE_TOML_STRING_SQ, Role::String},
            {SCE_TOML_STRING_DQ, Role::String}, {SCE_TOML_TRIPLE_STRING_SQ, Role::String},
            {SCE_TOML_TRIPLE_STRING_DQ, Role::String}, {SCE_TOML_STRINGEOL, Role::String},
            {SCE_TOML_ESCAPECHAR, Role::Constant}, {SCE_TOML_DATETIME, Role::Number},
        }, {"true false"}}},
        {QStringLiteral("markdown"), {"markdown", "Markdown", {
            {SCE_MARKDOWN_HEADER1, Role::Heading}, {SCE_MARKDOWN_HEADER2, Role::Heading},
            {SCE_MARKDOWN_HEADER3, Role::Heading}, {SCE_MARKDOWN_HEADER4, Role::Heading},
            {SCE_MARKDOWN_HEADER5, Role::Heading}, {SCE_MARKDOWN_HEADER6, Role::Heading},
            {SCE_MARKDOWN_STRONG1, Role::Constant}, {SCE_MARKDOWN_STRONG2, Role::Constant},
            {SCE_MARKDOWN_EM1, Role::Attribute}, {SCE_MARKDOWN_EM2, Role::Attribute},
            {SCE_MARKDOWN_ULIST_ITEM, Role::Operator}, {SCE_MARKDOWN_OLIST_ITEM, Role::Operator},
            {SCE_MARKDOWN_BLOCKQUOTE, Role::Comment}, {SCE_MARKDOWN_STRIKEOUT, Role::Comment},
            {SCE_MARKDOWN_HRULE, Role::Operator}, {SCE_MARKDOWN_LINK, Role::Function},
            {SCE_MARKDOWN_CODE, Role::String}, {SCE_MARKDOWN_CODE2, Role::String}, {SCE_MARKDOWN_CODEBK, Role::String},
        }, {}}},
        {QStringLiteral("css"), {"css", "CSS", {
            {SCE_CSS_TAG, Role::Tag}, {SCE_CSS_CLASS, Role::Type}, {SCE_CSS_PSEUDOCLASS, Role::Function},
            {SCE_CSS_UNKNOWN_PSEUDOCLASS, Role::Function}, {SCE_CSS_PSEUDOELEMENT, Role::Function},
            {SCE_CSS_OPERATOR, Role::Operator}, {SCE_CSS_IDENTIFIER, Role::Attribute},
            {SCE_CSS_IDENTIFIER2, Role::Attribute}, {SCE_CSS_IDENTIFIER3, Role::Attribute},
            {SCE_CSS_UNKNOWN_IDENTIFIER, Role::Attribute}, {SCE_CSS_VALUE, Role::Constant},
            {SCE_CSS_COMMENT, Role::Comment}, {SCE_CSS_ID, Role::Constant}, {SCE_CSS_IMPORTANT, Role::Keyword},
            {SCE_CSS_DIRECTIVE, Role::Preprocessor},
            {SCE_CSS_VARIABLE, Role::Preprocessor}, {SCE_CSS_DOUBLESTRING, Role::String},
            {SCE_CSS_SINGLESTRING, Role::String}, {SCE_CSS_ATTRIBUTE, Role::Attribute},
        }, {"color background background-color border border-radius display flex grid margin padding width height "
            "font font-size font-weight font-family line-height position top left right bottom z-index overflow "
            "opacity transition transform box-shadow align-items justify-content gap content cursor"}}},
        {QStringLiteral("sql"), {"sql", "SQL", {
            {SCE_SQL_COMMENT, Role::Comment}, {SCE_SQL_COMMENTLINE, Role::Comment}, {SCE_SQL_COMMENTDOC, Role::Comment},
            {SCE_SQL_COMMENTLINEDOC, Role::Comment}, {SCE_SQL_NUMBER, Role::Number}, {SCE_SQL_WORD, Role::Keyword},
            {SCE_SQL_WORD2, Role::Type}, {SCE_SQL_STRING, Role::String}, {SCE_SQL_CHARACTER, Role::String},
            {SCE_SQL_OPERATOR, Role::Operator}, {SCE_SQL_IDENTIFIER, Role::Variable},
            {SCE_SQL_QUOTEDIDENTIFIER, Role::Attribute},
        }, {"select from where insert into values update set delete create table alter drop index view join inner "
            "left right outer full on as and or not null is in exists between like group by order having limit "
            "offset distinct union all primary key foreign references default unique check constraint case when then "
            "else end begin commit rollback transaction returning with",
            "int integer bigint smallint serial bigserial text varchar char boolean bool date time timestamp "
            "timestamptz numeric decimal real float double json jsonb uuid bytea"}}},
        {QStringLiteral("cmake"), {"cmake", "CMake", {
            {SCE_CMAKE_COMMENT, Role::Comment}, {SCE_CMAKE_STRINGDQ, Role::String}, {SCE_CMAKE_STRINGLQ, Role::String},
            {SCE_CMAKE_STRINGRQ, Role::String}, {SCE_CMAKE_COMMANDS, Role::Function},
            {SCE_CMAKE_PARAMETERS, Role::Attribute}, {SCE_CMAKE_VARIABLE, Role::Constant},
            {SCE_CMAKE_STRINGVAR, Role::Constant}, {SCE_CMAKE_USERDEFINED, Role::Function},
            {SCE_CMAKE_WHILEDEF, Role::Keyword}, {SCE_CMAKE_FOREACHDEF, Role::Keyword},
            {SCE_CMAKE_IFDEFINEDEF, Role::Keyword}, {SCE_CMAKE_MACRODEF, Role::Keyword}, {SCE_CMAKE_NUMBER, Role::Number},
        }, {"add_compile_definitions add_compile_options add_custom_command add_custom_target add_definitions "
            "add_dependencies add_executable add_library add_subdirectory add_test cmake_minimum_required configure_file "
            "enable_testing file find_library find_package find_path find_program get_filename_component include "
            "include_directories install list message option project return set set_property set_target_properties "
            "set_tests_properties string target_compile_definitions target_compile_options target_include_directories "
            "target_link_libraries target_sources unset fetchcontent_declare fetchcontent_makeavailable "
            "qt_standard_project_setup qt_add_executable qt_generate_deploy_app_script"}}},
        {QStringLiteral("html"), {"hypertext", "HTML", {
            {SCE_H_TAG, Role::Tag}, {SCE_H_TAGUNKNOWN, Role::Tag}, {SCE_H_TAGEND, Role::Tag},
            {SCE_H_ATTRIBUTE, Role::Attribute}, {SCE_H_ATTRIBUTEUNKNOWN, Role::Attribute}, {SCE_H_NUMBER, Role::Number},
            {SCE_H_DOUBLESTRING, Role::String}, {SCE_H_SINGLESTRING, Role::String}, {SCE_H_VALUE, Role::String},
            {SCE_H_COMMENT, Role::Comment}, {SCE_H_ENTITY, Role::Constant}, {SCE_H_XMLSTART, Role::Preprocessor},
            {SCE_H_XMLEND, Role::Preprocessor}, {SCE_H_CDATA, Role::String}, {SCE_HJ_KEYWORD, Role::Keyword},
            {SCE_HJ_DOUBLESTRING, Role::String}, {SCE_HJ_SINGLESTRING, Role::String}, {SCE_HJ_COMMENT, Role::Comment},
            {SCE_HJ_COMMENTLINE, Role::Comment}, {SCE_HJ_NUMBER, Role::Number}, {SCE_HJ_SYMBOLS, Role::Operator},
            {SCE_HJ_WORD, Role::Variable},
        }, {"a abbr address article aside audio b body br button canvas code div dl dt em footer form h1 h2 h3 h4 h5 "
            "h6 head header hr html i iframe img input label li link main meta nav ol option p pre script section "
            "select small span strong style table tbody td template textarea th thead title tr ul video class id "
            "href src type name value rel content charset lang alt width height style", JsKeywords}}},
        {QStringLiteral("xml"), {"xml", "XML", {
            {SCE_H_TAG, Role::Tag}, {SCE_H_TAGUNKNOWN, Role::Tag}, {SCE_H_TAGEND, Role::Tag},
            {SCE_H_ATTRIBUTE, Role::Attribute}, {SCE_H_ATTRIBUTEUNKNOWN, Role::Attribute},
            {SCE_H_DOUBLESTRING, Role::String}, {SCE_H_SINGLESTRING, Role::String}, {SCE_H_COMMENT, Role::Comment},
            {SCE_H_ENTITY, Role::Constant}, {SCE_H_XMLSTART, Role::Preprocessor}, {SCE_H_XMLEND, Role::Preprocessor},
            {SCE_H_CDATA, Role::String},
        }, {}}},
    };
    return table;
}

QString languageKey(const QString &path)
{
    const QFileInfo info(path);
    const QString suffix = info.suffix().toLower();
    static const QHash<QString, QString> bySuffix = {
        {QStringLiteral("c"), QStringLiteral("c")}, {QStringLiteral("h"), QStringLiteral("cpp")},
        {QStringLiteral("cc"), QStringLiteral("cpp")}, {QStringLiteral("cpp"), QStringLiteral("cpp")},
        {QStringLiteral("cxx"), QStringLiteral("cpp")}, {QStringLiteral("hh"), QStringLiteral("cpp")},
        {QStringLiteral("hpp"), QStringLiteral("cpp")}, {QStringLiteral("hxx"), QStringLiteral("cpp")},
        {QStringLiteral("py"), QStringLiteral("python")}, {QStringLiteral("pyi"), QStringLiteral("python")},
        {QStringLiteral("js"), QStringLiteral("javascript")}, {QStringLiteral("jsx"), QStringLiteral("javascript")},
        {QStringLiteral("mjs"), QStringLiteral("javascript")}, {QStringLiteral("cjs"), QStringLiteral("javascript")},
        {QStringLiteral("ts"), QStringLiteral("typescript")}, {QStringLiteral("tsx"), QStringLiteral("typescript")},
        {QStringLiteral("java"), QStringLiteral("java")}, {QStringLiteral("cs"), QStringLiteral("csharp")},
        {QStringLiteral("kt"), QStringLiteral("kotlin")}, {QStringLiteral("kts"), QStringLiteral("kotlin")},
        {QStringLiteral("go"), QStringLiteral("go")}, {QStringLiteral("rs"), QStringLiteral("rust")},
        {QStringLiteral("sh"), QStringLiteral("bash")}, {QStringLiteral("bash"), QStringLiteral("bash")},
        {QStringLiteral("zsh"), QStringLiteral("bash")}, {QStringLiteral("ps1"), QStringLiteral("powershell")},
        {QStringLiteral("json"), QStringLiteral("json")}, {QStringLiteral("yaml"), QStringLiteral("yaml")},
        {QStringLiteral("yml"), QStringLiteral("yaml")}, {QStringLiteral("toml"), QStringLiteral("toml")},
        {QStringLiteral("md"), QStringLiteral("markdown")}, {QStringLiteral("markdown"), QStringLiteral("markdown")},
        {QStringLiteral("css"), QStringLiteral("css")}, {QStringLiteral("scss"), QStringLiteral("css")},
        {QStringLiteral("sql"), QStringLiteral("sql")}, {QStringLiteral("cmake"), QStringLiteral("cmake")},
        {QStringLiteral("html"), QStringLiteral("html")}, {QStringLiteral("htm"), QStringLiteral("html")},
        {QStringLiteral("vue"), QStringLiteral("html")}, {QStringLiteral("xml"), QStringLiteral("xml")},
        {QStringLiteral("ui"), QStringLiteral("xml")}, {QStringLiteral("qrc"), QStringLiteral("xml")},
        {QStringLiteral("svg"), QStringLiteral("xml")},
    };
    if (info.fileName() == QStringLiteral("CMakeLists.txt")) return QStringLiteral("cmake");
    return bySuffix.value(suffix);
}

QColor roleColor(Role role, const StudioTheme &theme)
{
    switch (role) {
    case Role::Comment: return theme.comment;
    case Role::Keyword: case Role::Heading: return theme.keyword;
    case Role::Type: return theme.type;
    case Role::String: return theme.string;
    case Role::Number: return theme.number;
    case Role::Function: return theme.function;
    case Role::Constant: return theme.constant;
    case Role::Operator: return theme.operatorColor;
    case Role::Preprocessor: return theme.preprocessor;
    case Role::Tag: return theme.tag;
    case Role::Attribute: return theme.attribute;
    case Role::Variable: return theme.variable;
    case Role::Error: return theme.danger;
    }
    return theme.variable;
}

Scintilla::sptr_t rgb(const QColor &color)
{
    return color.red() | (color.green() << 8) | (color.blue() << 16);
}

QString editorFontFamily()
{
    static const QString family = [] {
        const QStringList installed = QFontDatabase::families();
        for (const char *preferred : {"JetBrains Mono", "Cascadia Code", "Fira Code", "Source Code Pro", "Hack",
                 "SF Mono", "Menlo", "Consolas", "Ubuntu Mono", "DejaVu Sans Mono"}) {
            if (installed.contains(QString::fromLatin1(preferred))) return QString::fromLatin1(preferred);
        }
        return QFontInfo(QFontDatabase::systemFont(QFontDatabase::FixedFont)).family();
    }();
    return family;
}

void setMarker(ScintillaEditBase *editor, int number, int symbol, const StudioTheme &theme)
{
    editor->send(SCI_MARKERDEFINE, number, symbol);
    editor->send(SCI_MARKERSETFORE, number, rgb(theme.editorBackground));
    editor->send(SCI_MARKERSETBACK, number, rgb(theme.gutterText));
}

} // namespace

QString lexerNameForPath(const QString &path)
{
    const auto it = languages().constFind(languageKey(path));
    return it == languages().cend() ? QString() : QString::fromLatin1(it->lexer);
}

QString languageNameForPath(const QString &path)
{
    const auto it = languages().constFind(languageKey(path));
    return it == languages().cend() ? QStringLiteral("Texto plano") : QString::fromUtf8(it->name);
}

void applyEditorTheme(ScintillaEditBase *editor, const QString &path, const StudioTheme &theme)
{
    const QByteArray family = editorFontFamily().toUtf8();
    const int size = qMax(10, QFontDatabase::systemFont(QFontDatabase::FixedFont).pointSize());
    editor->send(SCI_STYLESETFONT, STYLE_DEFAULT, reinterpret_cast<Scintilla::sptr_t>(family.constData()));
    editor->send(SCI_STYLESETSIZE, STYLE_DEFAULT, size);
    editor->send(SCI_STYLESETFORE, STYLE_DEFAULT, rgb(theme.variable));
    editor->send(SCI_STYLESETBACK, STYLE_DEFAULT, rgb(theme.editorBackground));
    editor->send(SCI_STYLECLEARALL);

    const auto it = languages().constFind(languageKey(path));
    if (it != languages().cend()) {
        if (Scintilla::ILexer5 *lexer = CreateLexer(it->lexer)) {
            editor->send(SCI_SETILEXER, 0, reinterpret_cast<Scintilla::sptr_t>(lexer));
            for (size_t set = 0; set < it->keywords.size(); ++set)
                editor->send(SCI_SETKEYWORDS, set, reinterpret_cast<Scintilla::sptr_t>(it->keywords.at(set)));
            for (const auto &[style, role] : it->roles) {
                editor->send(SCI_STYLESETFORE, style, rgb(roleColor(role, theme)));
                if (role == Role::Comment) editor->send(SCI_STYLESETITALIC, style, 1);
                if (role == Role::Heading) editor->send(SCI_STYLESETBOLD, style, 1);
            }
            editor->send(SCI_COLOURISE, 0, -1);
        }
    }

    editor->send(SCI_STYLESETFORE, STYLE_LINENUMBER, rgb(theme.gutterText));
    editor->send(SCI_STYLESETBACK, STYLE_LINENUMBER, rgb(theme.editorBackground));
    editor->send(SCI_STYLESETFORE, STYLE_INDENTGUIDE, rgb(theme.indentGuide));
    editor->send(SCI_STYLESETBACK, STYLE_INDENTGUIDE, rgb(theme.editorBackground));
    editor->send(SCI_STYLESETFORE, STYLE_BRACELIGHT, rgb(theme.accent));
    editor->send(SCI_STYLESETBACK, STYLE_BRACELIGHT, rgb(theme.selection));
    editor->send(SCI_STYLESETBOLD, STYLE_BRACELIGHT, 1);
    editor->send(SCI_STYLESETFORE, STYLE_BRACEBAD, rgb(theme.danger));
    editor->send(SCI_SETCARETFORE, rgb(theme.caret));
    editor->send(SCI_SETCARETWIDTH, 2);
    editor->send(SCI_SETCARETLINEVISIBLE, 1);
    editor->send(SCI_SETCARETLINEBACK, rgb(theme.caretLine));
    editor->send(SCI_SETSELFORE, 0, 0);
    editor->send(SCI_SETSELBACK, 1, rgb(theme.selection));
    editor->send(SCI_SETWHITESPACEFORE, 1, rgb(theme.textFaint));
    editor->send(SCI_SETFOLDMARGINCOLOUR, 1, rgb(theme.editorBackground));
    editor->send(SCI_SETFOLDMARGINHICOLOUR, 1, rgb(theme.editorBackground));
    editor->send(SCI_SETMARGINBACKN, 1, rgb(theme.editorBackground));
    editor->send(SCI_SETEXTRAASCENT, 2);
    editor->send(SCI_SETEXTRADESCENT, 2);
    setMarker(editor, SC_MARKNUM_FOLDEROPEN, SC_MARK_ARROWDOWN, theme);
    setMarker(editor, SC_MARKNUM_FOLDER, SC_MARK_ARROW, theme);
    setMarker(editor, SC_MARKNUM_FOLDEREND, SC_MARK_ARROW, theme);
    setMarker(editor, SC_MARKNUM_FOLDEROPENMID, SC_MARK_ARROWDOWN, theme);
    for (int marker : {SC_MARKNUM_FOLDERSUB, SC_MARKNUM_FOLDERTAIL, SC_MARKNUM_FOLDERMIDTAIL})
        setMarker(editor, marker, SC_MARK_EMPTY, theme);
}

void updateBraceHighlight(ScintillaEditBase *editor)
{
    const auto position = editor->send(SCI_GETCURRENTPOS);
    auto isBrace = [editor](Scintilla::sptr_t at) {
        const int character = int(editor->send(SCI_GETCHARAT, static_cast<Scintilla::uptr_t>(at)));
        return character == '(' || character == ')' || character == '[' || character == ']'
            || character == '{' || character == '}';
    };
    Scintilla::sptr_t brace = -1;
    if (position > 0 && isBrace(position - 1)) brace = position - 1;
    else if (isBrace(position)) brace = position;
    if (brace < 0) {
        editor->send(SCI_BRACEHIGHLIGHT, static_cast<Scintilla::uptr_t>(-1), -1);
        return;
    }
    const auto match = editor->send(SCI_BRACEMATCH, static_cast<Scintilla::uptr_t>(brace), 0);
    if (match < 0) editor->send(SCI_BRACEBADLIGHT, static_cast<Scintilla::uptr_t>(brace));
    else editor->send(SCI_BRACEHIGHLIGHT, static_cast<Scintilla::uptr_t>(brace), match);
}
