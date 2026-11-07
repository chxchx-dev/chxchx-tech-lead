#pragma once

#include "agent_chat_cards.hpp"

class QLabel;
class QVBoxLayout;
class QWidget;

// "The agent was not allowed to do this": what it tried, why we think so, and one button per way of
// granting it. Answering replaces the buttons with the decision, so a card is answered once.
class ChatPermissionCard final : public ChatEntryCard {
    Q_OBJECT
public:
    ChatPermissionCard(const AgentChatModel::Entry &entry, QWidget *parent = nullptr);
    void refresh(const AgentChatModel::Entry &entry) override;
    bool isDecided() const { return m_decided; }
    // Index of the choice to click for tests and shortcuts; -1 is "not now".
    void choose(int choice);

signals:
    void choiceChosen(int choice);

private:
    QLabel *m_title = nullptr;
    QLabel *m_summary = nullptr;
    QLabel *m_detail = nullptr;
    QLabel *m_decision = nullptr;
    QWidget *m_buttons = nullptr;
    bool m_decided = false;
};
