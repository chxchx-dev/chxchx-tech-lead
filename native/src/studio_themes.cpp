#include "studio_theme.hpp"

namespace {

QColor c(const char *hex)
{
    return QColor(QString::fromLatin1(hex));
}

// Columns: background, surface, raised, hover, border, borderStrong, text,
// muted, faint, accent, accentStrong, onAccent, accentSoft, selectedText,
// success, warning, danger.
StudioTheme ui(const char *id, const char *name, const char *description, bool dark,
    std::initializer_list<const char *> colors)
{
    StudioTheme theme;
    theme.id = QString::fromLatin1(id);
    theme.name = QString::fromUtf8(name);
    theme.description = QString::fromUtf8(description);
    theme.dark = dark;
    QColor *targets[] = {&theme.background, &theme.surface, &theme.surfaceRaised, &theme.surfaceHover,
        &theme.border, &theme.borderStrong, &theme.text, &theme.textMuted, &theme.textFaint, &theme.accent,
        &theme.accentStrong, &theme.onAccent, &theme.accentSoft, &theme.selectedText, &theme.success,
        &theme.warning, &theme.danger};
    int index = 0;
    for (const char *color : colors) *targets[index++] = c(color);
    return theme;
}

// Columns: editorBackground, gutterText, gutterActive, caret, caretLine, selection, indentGuide.
void editor(StudioTheme &theme, std::initializer_list<const char *> colors)
{
    QColor *targets[] = {&theme.editorBackground, &theme.gutterText, &theme.gutterActive, &theme.caret,
        &theme.caretLine, &theme.selection, &theme.indentGuide};
    int index = 0;
    for (const char *color : colors) *targets[index++] = c(color);
}

// Columns: comment, keyword, type, string, number, function, constant,
// operator, preprocessor, tag, attribute, variable.
void syntax(StudioTheme &theme, std::initializer_list<const char *> colors)
{
    QColor *targets[] = {&theme.comment, &theme.keyword, &theme.type, &theme.string, &theme.number,
        &theme.function, &theme.constant, &theme.operatorColor, &theme.preprocessor, &theme.tag,
        &theme.attribute, &theme.variable};
    int index = 0;
    for (const char *color : colors) *targets[index++] = c(color);
}

QList<StudioTheme> buildThemes()
{
    QList<StudioTheme> themes;

    StudioTheme midnight = ui("medianoche", "Medianoche", "Azul profundo con acento cian. El tema original de ChxChx.", true,
        {"#0b1220", "#0e1728", "#111d30", "#17243a", "#1b2a40", "#263a55", "#e6edf7", "#91a7c2", "#5d7290",
         "#20c5d4", "#0e9aa8", "#ffffff", "#123846", "#8cf2f4", "#42be65", "#f1c21b", "#fa4d56"});
    editor(midnight, {"#0b1220", "#3f5573", "#a9bdd6", "#63e6ee", "#0f1a2c", "#1a3f55", "#18263a"});
    syntax(midnight, {"#5f7a99", "#c792ea", "#ffcb6b", "#a5e075", "#f78c6c", "#82aaff", "#ff9cac",
        "#89ddff", "#f07178", "#f07178", "#ffcb6b", "#dce8f7"});
    themes << midnight;

    StudioTheme graphite = ui("grafito", "Grafito", "Gris neutro y sobrio con acento azul, para jornadas largas.", true,
        {"#1e1f22", "#26282c", "#2b2d31", "#33363b", "#2e3135", "#43464c", "#dfe1e5", "#9da0a8", "#6f737a",
         "#4d9cff", "#3574f0", "#ffffff", "#2a3d63", "#d7e6ff", "#5fb865", "#e5b84b", "#f75464"});
    editor(graphite, {"#1e1f22", "#4b4f56", "#a8abb2", "#ced0d6", "#25272b", "#214283", "#2d3035"});
    syntax(graphite, {"#7a7e85", "#e0956c", "#6fb3d2", "#8fbf7f", "#2aacb8", "#56a8f5", "#c77dbb",
        "#bcbec4", "#b3ae60", "#d5b778", "#e0c88b", "#bcbec4"});
    themes << graphite;

    StudioTheme aurora = ui("aurora", "Aurora", "Violeta nocturno con rosa y verde neón.", true,
        {"#16121f", "#1d1829", "#231d31", "#2e2640", "#2a2238", "#3b3050", "#ece6f7", "#a79cc0", "#6f6589",
         "#c084fc", "#9b5de5", "#ffffff", "#3a2958", "#f0dcff", "#4ade80", "#facc15", "#fb7185"});
    editor(aurora, {"#16121f", "#4f4568", "#b8acd4", "#f0abfc", "#1d1729", "#3d2c5e", "#251e33"});
    syntax(aurora, {"#6e6589", "#ff79c6", "#7dd3fc", "#f6e3a1", "#bd93f9", "#7ee8a2", "#ffb86c",
        "#ff92d0", "#ff79c6", "#ff79c6", "#7ee8a2", "#ece6f7"});
    themes << aurora;

    StudioTheme forest = ui("bosque", "Bosque", "Verde oscuro y sereno con acentos cálidos.", true,
        {"#0f1a17", "#13211d", "#182924", "#1f332c", "#1d2e28", "#2a4239", "#e2efe9", "#93b3a6", "#5f7f72",
         "#34d399", "#10a37f", "#ffffff", "#15473a", "#a7f3d0", "#4ade80", "#f4c95d", "#f87171"});
    editor(forest, {"#0f1a17", "#3e5a4f", "#a3c4b6", "#6ee7b7", "#13221d", "#1d4a3d", "#1a2b25"});
    syntax(forest, {"#5d7d70", "#f4a261", "#e9c46a", "#b5e48c", "#f28482", "#7fd1ff", "#ffadad",
        "#8ecae6", "#f4a261", "#e76f51", "#e9c46a", "#e2efe9"});
    themes << forest;

    StudioTheme amber = ui("ambar", "Ámbar", "Tonos tierra cálidos con acento naranja.", true,
        {"#1a1410", "#211913", "#282018", "#33281f", "#2e241b", "#44362a", "#f3e9df", "#bba591", "#85705d",
         "#f59e0b", "#d97706", "#ffffff", "#4a3214", "#fde68a", "#84cc16", "#facc15", "#f87171"});
    editor(amber, {"#1a1410", "#5c4a3b", "#cdb8a3", "#fbbf24", "#211913", "#4d3518", "#2c2219"});
    syntax(amber, {"#8a7360", "#fb923c", "#facc15", "#a3e635", "#f472b6", "#60a5fa", "#f87171",
        "#fcd34d", "#fb7185", "#fb923c", "#facc15", "#f3e9df"});
    themes << amber;

    StudioTheme light = ui("claro", "Claro", "Luminoso y limpio, con acento azul.", false,
        {"#f5f7fa", "#ffffff", "#ffffff", "#eef1f6", "#e3e7ee", "#cdd4df", "#1f2937", "#5b6575", "#94a0b2",
         "#2563eb", "#1d4ed8", "#ffffff", "#dbe7ff", "#1e3a8a", "#16a34a", "#b45309", "#dc2626"});
    editor(light, {"#ffffff", "#a0aab8", "#4b5563", "#1d4ed8", "#f4f7fc", "#cfe0ff", "#e8ecf2"});
    syntax(light, {"#7c8796", "#8b2bd9", "#b45309", "#15803d", "#c2410c", "#1d4ed8", "#be185d",
        "#475569", "#a21caf", "#b91c1c", "#b45309", "#1f2937"});
    themes << light;

    return themes;
}

} // namespace

const QList<StudioTheme> &studioThemes()
{
    static const QList<StudioTheme> themes = buildThemes();
    return themes;
}

const StudioTheme &studioThemeById(const QString &id)
{
    for (const StudioTheme &theme : studioThemes()) {
        if (theme.id == id) return theme;
    }
    return studioThemes().constFirst();
}
