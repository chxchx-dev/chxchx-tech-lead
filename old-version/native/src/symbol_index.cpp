#include "symbol_index.hpp"

#include <QRegularExpression>
#include <QStringList>

namespace {

QList<QRegularExpression> patternsFor(const QString &suffix)
{
    static const QRegularExpression pythonFunction(
        QStringLiteral("^\\s*(?:async\\s+)?def\\s+([A-Za-z_]\\w*)\\s*\\("));
    static const QRegularExpression pythonClass(
        QStringLiteral(R"(^\s*class\s+([A-Za-z_]\w*)\b)"));
    static const QRegularExpression scriptDeclaration(
        QStringLiteral(R"(^\s*(?:export\s+)?(?:default\s+)?(?:async\s+)?(?:abstract\s+)?(class|function|interface|type|enum)\s+([A-Za-z_$][\w$]*)\b)"));
    static const QRegularExpression scriptVariable(
        QStringLiteral(R"(^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:async\s*)?(?:\([^)]*\)\s*=>|function\b))"));
    static const QRegularExpression nativeType(
        QStringLiteral(R"(^\s*(class|struct|enum|namespace|interface)\s+([A-Za-z_]\w*)\b)"));
    static const QRegularExpression nativeFunction(
        QStringLiteral(R"(^\s*(?:[\w:<>,*&~]+\s+)+(~?[A-Za-z_]\w*)\s*\([^;]*\)\s*(?:const\s*)?(?:noexcept\s*)?\{)"));

    const QString language = suffix.toLower();
    if (language == QStringLiteral("py")) return {pythonClass, pythonFunction};
    if (language == QStringLiteral("js") || language == QStringLiteral("jsx")
        || language == QStringLiteral("ts") || language == QStringLiteral("tsx")) {
        return {scriptDeclaration, scriptVariable};
    }
    if (language == QStringLiteral("c") || language == QStringLiteral("h")
        || language == QStringLiteral("cc") || language == QStringLiteral("cpp")
        || language == QStringLiteral("cxx") || language == QStringLiteral("hh")
        || language == QStringLiteral("hpp") || language == QStringLiteral("hxx")
        || language == QStringLiteral("java") || language == QStringLiteral("cs")) {
        return {nativeType, nativeFunction};
    }
    return {};
}

bool controlKeyword(const QString &name)
{
    static const QStringList keywords = {QStringLiteral("if"), QStringLiteral("for"),
        QStringLiteral("while"), QStringLiteral("switch"), QStringLiteral("catch"),
        QStringLiteral("return"), QStringLiteral("sizeof"), QStringLiteral("decltype")};
    return keywords.contains(name);
}

} // namespace

QList<SourceSymbol> SymbolIndex::scan(const QString &text, const QString &suffix)
{
    const QList<QRegularExpression> patterns = patternsFor(suffix);
    QList<SourceSymbol> symbols;
    if (patterns.isEmpty()) return symbols;

    const QStringList lines = text.split(QLatin1Char('\n'));
    for (qsizetype index = 0; index < lines.size(); ++index) {
        for (const QRegularExpression &pattern : patterns) {
            const QRegularExpressionMatch match = pattern.match(lines.at(index));
            if (!match.hasMatch()) continue;
            const QString name = match.lastCapturedIndex() > 1
                ? match.captured(2) : match.captured(1);
            if (name.isEmpty() || controlKeyword(name)) break;
            QString kind = QStringLiteral("función");
            if (pattern == patterns.first() && suffix.compare(QStringLiteral("py"), Qt::CaseInsensitive) == 0) {
                kind = QStringLiteral("clase");
            } else if (match.lastCapturedIndex() > 1) {
                const QString possibleKind = match.captured(1);
                if (possibleKind == QStringLiteral("class") || possibleKind == QStringLiteral("struct")
                    || possibleKind == QStringLiteral("enum") || possibleKind == QStringLiteral("namespace")
                    || possibleKind == QStringLiteral("interface") || possibleKind == QStringLiteral("function")) {
                    kind = possibleKind;
                }
            }
            symbols.append({name, kind, static_cast<int>(index + 1)});
            break;
        }
    }
    return symbols;
}
