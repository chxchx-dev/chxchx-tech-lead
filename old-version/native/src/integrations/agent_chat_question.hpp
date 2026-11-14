#pragma once

#include "agent_chat_cards.hpp"

#include <QJsonObject>

class QLabel;
class QLineEdit;
class QPushButton;
class QVBoxLayout;
class QWidget;

// "The agent asks you something": one block per question with its options as buttons, plus a free answer.
// A single question with single choice answers on click; otherwise the answers go with "Responder".
class ChatQuestionCard final : public ChatEntryCard {
    Q_OBJECT
public:
    ChatQuestionCard(const AgentChatModel::Entry &entry, QWidget *parent = nullptr);
    void refresh(const AgentChatModel::Entry &entry) override;
    bool isDecided() const { return m_decided; }
    // For tests and shortcuts: pick an option (or write a free answer) of one question.
    void select(int question, const QString &answer);
    void submit();

signals:
    // `answers` maps each question's text to the chosen label(s); empty means "not now".
    void answered(const QJsonObject &answers);

private:
    struct Block {
        QString question;
        bool multi = false;
        QStringList chosen;
        QList<QPushButton *> buttons;
        QLineEdit *free = nullptr;
    };
    void update();

    QList<Block> m_blocks;
    QLabel *m_decision = nullptr;
    QWidget *m_body = nullptr;
    QPushButton *m_submit = nullptr;
    bool m_decided = false;
    bool m_instant = false;
};
