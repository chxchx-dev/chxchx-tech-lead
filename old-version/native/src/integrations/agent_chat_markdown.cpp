#include "agent_chat_markdown.hpp"

#include <QRegularExpression>
#include <QStringList>
#include <QTextDocument>

QList<ChatSegment> splitFencedCode(const QString &markdown)
{
    QList<ChatSegment> segments;
    QStringList prose;
    QStringList code;
    QString language;
    bool inCode = false;
    const auto flushProse = [&] {
        const QString text = prose.join(QLatin1Char('\n')).trimmed();
        if (!text.isEmpty()) segments.append({false, {}, text});
        prose.clear();
    };
    for (const QString &line : markdown.split(QLatin1Char('\n'))) {
        const QString trimmed = line.trimmed();
        if (trimmed.startsWith(QStringLiteral("```"))) {
            if (!inCode) {
                flushProse();
                language = trimmed.mid(3).trimmed();
                inCode = true;
            } else {
                segments.append({true, language, code.join(QLatin1Char('\n'))});
                code.clear();
                inCode = false;
            }
            continue;
        }
        (inCode ? code : prose).append(line);
    }
    // An unterminated fence still shows as code instead of disappearing.
    if (inCode) segments.append({true, language, code.join(QLatin1Char('\n'))});
    flushProse();
    return segments;
}

QString chatMarkdownToHtml(const QString &markdown)
{
    QTextDocument document;
    document.setMarkdown(markdown, QTextDocument::MarkdownFeatures(
        QTextDocument::MarkdownDialectGitHub | QTextDocument::MarkdownNoHTML));
    const QString html = document.toHtml();
    static const QRegularExpression body(QStringLiteral("<body[^>]*>(.*)</body>"),
        QRegularExpression::DotMatchesEverythingOption);
    const auto match = body.match(html);
    return match.hasMatch() ? match.captured(1) : markdown.toHtmlEscaped();
}
