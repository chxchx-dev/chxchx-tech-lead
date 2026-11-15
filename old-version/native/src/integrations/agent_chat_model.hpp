#pragma once

#include <QJsonArray>
#include <QJsonObject>
#include <QList>
#include <QString>
#include <QStringList>

// Transcript of a visual agent chat built from `chxchx.agent-event` v1 lines
// emitted by `chxchx-tech agent chat`. It owns no process and no widgets so it
// can be tested headless; AgentChatView renders it.
class AgentChatModel final {
public:
    enum class Kind { User, Assistant, Reasoning, Tool, Notice, Error, Permission, Question };

    struct Entry {
        Kind kind = Kind::Notice;
        QString text;
        QString toolId;
        QString toolKind;
        QString status;
        QString output;
        QStringList images;
        // Permission cards: what the agent was not allowed to do, how sure we are and what the person picked.
        QJsonArray choices;
        bool certain = true;
        QString decision;
        // Live requests (Claude): the agent is paused until Studio answers this id. Empty for after-the-fact cards.
        QString requestId;
        // Question cards: [{question, header, multiSelect, options: [{label, description}]}].
        QJsonArray questions;
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
    // Records what the person answered to a permission card (an empty label means "not now").
    void decidePermission(qsizetype index, const QString &label);
    // True while a live request is waiting for an answer.
    bool awaitingAnswer() const;
    // The turn ended: live requests nobody answered can no longer be answered.
    void expireLiveRequests();

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
