#pragma once

#include "agent_chat_model.hpp"

#include <QWidget>

class QLabel;
class QProgressBar;

// Compact meters: context used, the plan's 5 h and weekly windows, and what
// this session has spent. Each meter hides until its data arrives.
class ChatUsageBar final : public QWidget {
    Q_OBJECT
public:
    explicit ChatUsageBar(QWidget *parent = nullptr);
    void setUsage(const AgentChatModel::Usage &usage, const QString &spend);
    // Text of every visible meter, for tests and accessibility.
    QString summary() const;

private:
    struct Meter {
        QWidget *pill = nullptr;
        QLabel *label = nullptr;
        QProgressBar *bar = nullptr;
        QLabel *value = nullptr;
    };
    Meter addMeter(const QString &label);
    static void showMeter(const Meter &meter, double percent, const QString &value, const QString &tooltip);

    Meter m_context;
    Meter m_primary;
    Meter m_secondary;
    QLabel *m_spend = nullptr;
};
