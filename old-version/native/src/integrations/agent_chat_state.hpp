#pragma once

#include <QList>
#include <QPair>
#include <QString>

#include <functional>

class QObject;

// The conversation a visual chat saved the last time it was used (`chxchx-tech agent chat-state`).
// Python writes it after every turn, so closing the tab or Studio loses nothing.
struct ChatSavedState {
    QString sessionId;
    QList<QPair<QString, QString>> messages; // role ("user" / "assistant"), text
};

// Reads the saved conversation and marks its session to be checked by the next turn. `done` runs only when
// there is something to restore; `context` owns the work (it stops with it).
void loadChatState(const QString &cliProgram, const QString &projectPath, const QString &agentId, QObject *context,
    const std::function<void(const ChatSavedState &)> &done);
// A new chat starts from nothing: forgets the saved conversation.
void resetChatState(const QString &cliProgram, const QString &projectPath, const QString &agentId);
