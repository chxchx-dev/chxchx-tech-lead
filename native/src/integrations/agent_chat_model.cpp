#include "agent_chat_model.hpp"

#include <QJsonValue>

namespace {

constexpr int AgentEventVersion = 1;

AgentChatModel::LimitWindow parseWindow(const QJsonValue &value)
{
    AgentChatModel::LimitWindow window;
    const QJsonObject object = value.toObject();
    if (!object.value(QStringLiteral("used_percent")).isDouble()) return window;
    window.usedPercent = object.value(QStringLiteral("used_percent")).toDouble();
    window.windowMinutes = object.value(QStringLiteral("window_minutes")).toInt();
    window.resetsAt = object.value(QStringLiteral("resets_at")).toInteger();
    return window;
}

} // namespace

QString compactTokens(qint64 tokens)
{
    if (tokens >= 1000000) return QStringLiteral("%1M").arg(tokens / 1000000.0, 0, 'f', 1);
    if (tokens >= 1000) return QStringLiteral("%1k").arg(qRound64(tokens / 1000.0));
    return QString::number(tokens);
}

void AgentChatModel::addUserMessage(const QString &text, const QStringList &images)
{
    Entry entry;
    entry.kind = Kind::User;
    entry.text = text;
    entry.images = images;
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
        // Each turn is a separate run, so the session total is the running sum.
        m_usage.inputTokens += event.value(QStringLiteral("input_tokens")).toInteger();
        m_usage.outputTokens += event.value(QStringLiteral("output_tokens")).toInteger();
        const auto cost = event.value(QStringLiteral("cost_usd"));
        if (cost.isDouble()) {
            m_usage.costUsd += cost.toDouble();
            m_usage.hasCost = true;
        }
        return true;
    }
    if (type == QStringLiteral("limits")) {
        if (event.value(QStringLiteral("context_tokens")).isDouble())
            m_usage.contextTokens = event.value(QStringLiteral("context_tokens")).toInteger();
        if (event.value(QStringLiteral("context_window")).isDouble())
            m_usage.contextWindow = event.value(QStringLiteral("context_window")).toInteger();
        const LimitWindow primary = parseWindow(event.value(QStringLiteral("primary")));
        const LimitWindow secondary = parseWindow(event.value(QStringLiteral("secondary")));
        if (primary.usedPercent >= 0) m_usage.primary = primary;
        if (secondary.usedPercent >= 0) m_usage.secondary = secondary;
        if (event.value(QStringLiteral("plan")).isString()) m_usage.plan = event.value(QStringLiteral("plan")).toString();
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
            ++it->revision;
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
    m_usage = {};
}

QString AgentChatModel::usageSummary() const
{
    if (m_usage.inputTokens == 0 && m_usage.outputTokens == 0) return {};
    QString text = QStringLiteral("%1 de entrada · %2 de salida")
        .arg(compactTokens(m_usage.inputTokens), compactTokens(m_usage.outputTokens));
    if (m_usage.hasCost) text += QStringLiteral(" · US$ %1").arg(m_usage.costUsd, 0, 'f', 4);
    return text;
}
