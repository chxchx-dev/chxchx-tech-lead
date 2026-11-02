#pragma once

#include <QJsonObject>
#include <QList>
#include <QString>

// Transcript of a visual agent chat built from `chxchx.agent-event` v1 lines
// emitted by `chxchx-tech agent chat`. It owns no process and no widgets so the
// rendering can be tested headless.
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
        mutable QString cachedHtml;
    };

    void addUserMessage(const QString &text);
    // Applies one decoded event; returns false for unknown or foreign schemas.
    bool apply(const QJsonObject &event);
    void clear();

    QString renderHtml() const;
    const QList<Entry> &entries() const { return m_entries; }
    QString sessionId() const { return m_sessionId; }
    void resetSession() { m_sessionId.clear(); }
    QString usageSummary() const { return m_usage; }

private:
    QString entryHtml(const Entry &entry) const;

    QList<Entry> m_entries;
    QString m_sessionId;
    QString m_usage;
};
