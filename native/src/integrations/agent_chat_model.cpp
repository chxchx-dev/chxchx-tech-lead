#include "agent_chat_model.hpp"

#include <QRegularExpression>
#include <QTextDocument>

namespace {

constexpr int AgentEventVersion = 1;
constexpr qsizetype MaxPreviewChars = 700;
constexpr qsizetype MaxPreviewLines = 8;

QString markdownToHtml(const QString &markdown)
{
    QTextDocument document;
    document.setMarkdown(markdown);
    const QString html = document.toHtml();
    static const QRegularExpression body(QStringLiteral("<body[^>]*>(.*)</body>"),
        QRegularExpression::DotMatchesEverythingOption);
    const auto match = body.match(html);
    return match.hasMatch() ? match.captured(1) : html.toHtmlEscaped();
}

// Tool output is a preview: long listings would bury the conversation.
QString outputPreview(const QString &output)
{
    const QStringList lines = output.trimmed().split(QLatin1Char('\n'));
    QString preview = lines.mid(0, MaxPreviewLines).join(QLatin1Char('\n'));
    if (preview.size() > MaxPreviewChars) preview = preview.left(MaxPreviewChars) + QStringLiteral("…");
    if (lines.size() > MaxPreviewLines)
        preview += QStringLiteral("\n… %1 líneas más").arg(lines.size() - MaxPreviewLines);
    return preview;
}

QString toolSymbol(const QString &kind)
{
    if (kind == QStringLiteral("command")) return QStringLiteral("$");
    if (kind == QStringLiteral("file_change")) return QStringLiteral("✎");
    if (kind == QStringLiteral("search")) return QStringLiteral("⌕");
    if (kind == QStringLiteral("todo")) return QStringLiteral("☐");
    return QStringLiteral("⚙");
}

QString statusBadge(const QString &status)
{
    if (status == QStringLiteral("completed"))
        return QStringLiteral("<span style='color:#42be65;'>✓</span>");
    if (status == QStringLiteral("failed"))
        return QStringLiteral("<span style='color:#fa4d56;'>✗</span>");
    return QStringLiteral("<span style='color:#f1c21b;'>…</span>");
}

QString card(const QString &background, const QString &border, const QString &content)
{
    return QStringLiteral("<table width='100%' cellpadding='8' cellspacing='0' "
                          "style='margin:4px 0; background-color:%1; border-left:3px solid %2;'>"
                          "<tr><td>%3</td></tr></table>")
        .arg(background, border, content);
}

} // namespace

void AgentChatModel::addUserMessage(const QString &text)
{
    Entry entry;
    entry.kind = Kind::User;
    entry.text = text;
    m_entries.append(entry);
}

bool AgentChatModel::apply(const QJsonObject &event)
{
    if (event.value(QStringLiteral("schema")).toString() != QStringLiteral("chxchx.agent-event")
        || event.value(QStringLiteral("schema_version")).toInt(-1) != AgentEventVersion) {
        return false;
    }
    const QString type = event.value(QStringLiteral("type")).toString();
    Entry entry;
    if (type == QStringLiteral("session") || type == QStringLiteral("done")) {
        const QString session = event.value(QStringLiteral("session_id")).toString();
        if (!session.isEmpty()) m_sessionId = session;
        return true;
    }
    if (type == QStringLiteral("usage")) {
        const auto input = event.value(QStringLiteral("input_tokens")).toInteger();
        const auto output = event.value(QStringLiteral("output_tokens")).toInteger();
        m_usage = QStringLiteral("%1 tokens de entrada · %2 de salida").arg(input).arg(output);
        const auto cost = event.value(QStringLiteral("cost_usd"));
        if (cost.isDouble()) m_usage += QStringLiteral(" · US$ %1").arg(cost.toDouble(), 0, 'f', 4);
        return true;
    }
    if (type == QStringLiteral("message")) {
        entry.kind = Kind::Assistant;
        entry.text = event.value(QStringLiteral("text")).toString();
    } else if (type == QStringLiteral("reasoning")) {
        entry.kind = Kind::Reasoning;
        entry.text = event.value(QStringLiteral("text")).toString();
    } else if (type == QStringLiteral("notice")) {
        entry.kind = event.value(QStringLiteral("error")).toBool() ? Kind::Error : Kind::Notice;
        entry.text = event.value(QStringLiteral("message")).toString();
    } else if (type == QStringLiteral("error")) {
        entry.kind = Kind::Error;
        entry.text = event.value(QStringLiteral("message")).toString();
    } else if (type == QStringLiteral("tool")) {
        const QString id = event.value(QStringLiteral("id")).toString();
        // Later events for the same tool update its card in place.
        for (auto it = m_entries.rbegin(); !id.isEmpty() && it != m_entries.rend(); ++it) {
            if (it->kind != Kind::Tool || it->toolId != id) continue;
            it->status = event.value(QStringLiteral("status")).toString();
            const QString output = event.value(QStringLiteral("output")).toString();
            if (!output.isEmpty()) it->output = output;
            const QString title = event.value(QStringLiteral("title")).toString();
            if (!title.isEmpty()) it->text = title;
            it->cachedHtml.clear();
            return true;
        }
        entry.kind = Kind::Tool;
        entry.toolId = id;
        entry.toolKind = event.value(QStringLiteral("kind")).toString();
        entry.text = event.value(QStringLiteral("title")).toString();
        entry.status = event.value(QStringLiteral("status")).toString();
        entry.output = event.value(QStringLiteral("output")).toString();
    } else {
        return type == QStringLiteral("start") || type == QStringLiteral("plan");
    }
    if (entry.text.trimmed().isEmpty() && entry.kind != Kind::Tool) return true;
    m_entries.append(entry);
    return true;
}

void AgentChatModel::clear()
{
    m_entries.clear();
    m_sessionId.clear();
    m_usage.clear();
}

QString AgentChatModel::renderHtml() const
{
    QString html = QStringLiteral("<body style='color:#e0e6f0;'>");
    for (const Entry &entry : m_entries) {
        if (entry.cachedHtml.isEmpty()) entry.cachedHtml = entryHtml(entry);
        html += entry.cachedHtml;
    }
    return html + QStringLiteral("</body>");
}

QString AgentChatModel::entryHtml(const Entry &entry) const
{
    switch (entry.kind) {
    case Kind::User:
        return card(QStringLiteral("#1c2a44"), QStringLiteral("#78a9ff"),
            QStringLiteral("<b style='color:#78a9ff;'>Tú</b><br>%1")
                .arg(entry.text.toHtmlEscaped().replace(QLatin1Char('\n'), QStringLiteral("<br>"))));
    case Kind::Assistant:
        return card(QStringLiteral("#161b22"), QStringLiteral("#33b1ff"),
            QStringLiteral("<b style='color:#33b1ff;'>Agente</b>%1").arg(markdownToHtml(entry.text)));
    case Kind::Reasoning:
        return QStringLiteral("<p style='color:#8d96a8; font-style:italic; margin:2px 0;'>💭 %1</p>")
            .arg(entry.text.toHtmlEscaped());
    case Kind::Tool: {
        QString content = QStringLiteral("%1 <code style='color:#c6c6c6;'>%2 %3</code>")
            .arg(statusBadge(entry.status), toolSymbol(entry.toolKind), entry.text.toHtmlEscaped());
        if (!entry.output.trimmed().isEmpty()) {
            content += QStringLiteral("<pre style='color:#a8b3c4; margin:6px 0 0 0; white-space:pre-wrap;'>%1</pre>")
                .arg(outputPreview(entry.output).toHtmlEscaped());
        }
        return card(QStringLiteral("#11151c"), QStringLiteral("#4d5358"), content);
    }
    case Kind::Notice:
        return QStringLiteral("<p style='color:#8d96a8; margin:2px 0;'>ℹ %1</p>").arg(entry.text.toHtmlEscaped());
    case Kind::Error:
        return card(QStringLiteral("#2d1215"), QStringLiteral("#fa4d56"),
            QStringLiteral("<b style='color:#fa4d56;'>Error</b><br>%1").arg(entry.text.toHtmlEscaped()));
    }
    return {};
}
