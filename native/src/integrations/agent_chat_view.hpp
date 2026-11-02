#pragma once

#include "agent_chat_cards.hpp"

#include <QList>
#include <QScrollArea>

class QLabel;
class QTimer;
class QVBoxLayout;

// Scrollable transcript built from native cards. It follows the bottom while
// the user is there, animates running tools and shows a welcome screen with
// suggestions when the chat is empty.
class AgentChatView final : public QScrollArea {
    Q_OBJECT
public:
    explicit AgentChatView(QWidget *parent = nullptr);

    void setIdentity(const ChatIdentity &identity);
    // Creates cards for new entries and refreshes the ones whose revision changed.
    void sync(const AgentChatModel &model);
    void setBusy(bool busy, const QString &label = {});
    int cardCount() const { return int(m_cards.size()); }
    ChatEntryCard *card(int index) const { return m_cards.value(index); }

signals:
    void suggestionChosen(const QString &text);

protected:
    void resizeEvent(QResizeEvent *event) override;

private:
    void clearCards();
    void rebuildWelcome();
    void animate();

    ChatIdentity m_identity;
    QWidget *m_content = nullptr;
    QVBoxLayout *m_layout = nullptr;
    QWidget *m_welcome = nullptr;
    QWidget *m_typing = nullptr;
    QLabel *m_typingAvatar = nullptr;
    QLabel *m_typingLabel = nullptr;
    QString m_busyLabel;
    QList<ChatEntryCard *> m_cards;
    QList<int> m_revisions;
    QTimer *m_animation = nullptr;
    int m_frame = 0;
    bool m_stickToBottom = true;
};
