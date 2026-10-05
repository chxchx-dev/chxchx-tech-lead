#include "agent_session_widget.hpp"

#include "pty_session.hpp"
#include "vt_terminal_widget.hpp"

#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QTimer>
#include <QVBoxLayout>

AgentSessionWidget::AgentSessionWidget(QString agentId, QString program, QStringList arguments,
    QString workingDirectory, QWidget *parent)
    : QWidget(parent), m_agentId(std::move(agentId))
{
    auto *layout = new QVBoxLayout(this);
    auto *toolbar = new QHBoxLayout();
    m_status = new QLabel(QStringLiteral("Iniciando %1…").arg(m_agentId), this);
    m_stop = new QPushButton(QStringLiteral("Detener agente"), this);
    toolbar->addWidget(m_status, 1);
    toolbar->addWidget(m_stop);
    layout->addLayout(toolbar);
    m_terminal = new VtTerminalWidget(this);
    layout->addWidget(m_terminal, 1);

    m_pty = new PtySession(this);
    connect(m_pty, &PtySession::outputReceived, m_terminal, &VtTerminalWidget::feed);
    connect(m_terminal, &VtTerminalWidget::inputReady, m_pty, &PtySession::write);
    connect(m_terminal, &VtTerminalWidget::terminalResized, m_pty, &PtySession::resize);
    connect(m_pty, &PtySession::processFinished, this, [this](int exitCode) {
        m_status->setText(QStringLiteral("%1 terminó · código %2").arg(m_agentId).arg(exitCode));
        m_stop->setEnabled(false);
    });
    connect(m_pty, &PtySession::failed, this, [this](const QString &message) {
        m_status->setText(message);
        focusTerminal();
    });
    connect(m_stop, &QPushButton::clicked, this, [this] {
        m_pty->stop();
        m_status->setText(QStringLiteral("%1 detenido").arg(m_agentId));
        m_stop->setEnabled(false);
    });

    QString error;
    if (m_pty->start(program, arguments, workingDirectory, 110, 32, &error)) {
        m_status->setText(QStringLiteral("%1 activo · terminal integrada").arg(m_agentId));
        focusTerminal();
    } else {
        m_status->setText(QStringLiteral("No se pudo iniciar %1: %2").arg(m_agentId, error));
        m_stop->setEnabled(false);
    }
}

AgentSessionWidget::~AgentSessionWidget()
{
    if (m_pty) m_pty->stop();
}

QString AgentSessionWidget::agentId() const { return m_agentId; }

void AgentSessionWidget::focusTerminal()
{
    QTimer::singleShot(0, m_terminal, [terminal = m_terminal] {
        terminal->setFocus(Qt::OtherFocusReason);
    });
}
