#pragma once

#include <QWidget>
#include <QStringList>

class QLabel;
class QPushButton;
class PtySession;
class VtTerminalWidget;

class AgentSessionWidget final : public QWidget {
    Q_OBJECT
public:
    AgentSessionWidget(QString agentId, QString program, QStringList arguments,
        QString workingDirectory, QWidget *parent = nullptr);
    ~AgentSessionWidget() override;

    QString agentId() const;
    bool isRunning() const;
    void focusTerminal();

private:
    QString m_agentId;
    PtySession *m_pty = nullptr;
    VtTerminalWidget *m_terminal = nullptr;
    QLabel *m_status = nullptr;
    QPushButton *m_stop = nullptr;
};
