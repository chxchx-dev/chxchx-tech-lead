#pragma once

#include <QJsonObject>
#include <QList>
#include <QString>
#include <QStringList>

// Transcript of a visual agent chat built from `chxchx.agent-event` v1 lines
// emitted by `chxchx-tech agent chat`. It owns no process and no widgets so it
// can be tested headless; AgentChatView renders it.
class AgentChatModel final {
public:
    enum class Kind { User, Assistant, Reasoning, Tool, Notice, Error };

    struct Entry {
        Kind kind = Kind::Notice;
        QString text;
        QString toolId;
        QString toolKind;
        QString status;
        QString output;
        QStringList images;
        // Bumped when a tool event updates the entry in place.
        int revision = 0;
    };

    struct LimitWindow {
        double usedPercent = -1;
        int windowMinutes = 0;
        qint64 resetsAt = 0;
    };

    // What the session has spent and how much room is left.
    struct Usage {
        qint64 inputTokens = 0;
        qint64 outputTokens = 0;
        double costUsd = 0;
        bool hasCost = false;
        qint64 contextTokens = 0;
        qint64 contextWindow = 0;
        LimitWindow primary;
        LimitWindow secondary;
        QString plan;
    };

    void addUserMessage(const QString &text, const QStringList &images = {});
    // Applies one decoded event; returns false for unknown or foreign schemas.
    bool apply(const QJsonObject &event);
    void clear();

    const QList<Entry> &entries() const { return m_entries; }
    QString sessionId() const { return m_sessionId; }
    void resetSession() { m_sessionId.clear(); }
    const Usage &usage() const { return m_usage; }
    // Session spend, e.g. "40k de entrada · 1k de salida · US$ 0.12".
    QString usageSummary() const;

private:
    QList<Entry> m_entries;
    QString m_sessionId;
    Usage m_usage;
};

// 1234 -> "1k", 1500000 -> "1.5M".
QString compactTokens(qint64 tokens);
