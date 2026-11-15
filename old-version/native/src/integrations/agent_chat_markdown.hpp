#pragma once

#include <QList>
#include <QString>

// Splits agent replies into prose and fenced code so code can render as its
// own block (monospace, language label, copy button).
struct ChatSegment {
    bool code = false;
    QString language;
    QString text;
};

QList<ChatSegment> splitFencedCode(const QString &markdown);
// Markdown to Qt rich text with raw HTML disabled, so replies cannot inject markup.
QString chatMarkdownToHtml(const QString &markdown);
