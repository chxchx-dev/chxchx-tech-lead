#include "workspace_terminal_widget.hpp"

#include "pty_session.hpp"
#include "vt_terminal_widget.hpp"

#include <QDir>
#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QVBoxLayout>

WorkspaceTerminalWidget::WorkspaceTerminalWidget(QString workingDirectory, QWidget *parent)
    : QWidget(parent)
{
    auto *layout = new QVBoxLayout(this);
    auto *toolbar = new QHBoxLayout();
    m_status = new QLabel(QStringLiteral("Iniciando terminal…"), this);
    auto *stop = new QPushButton(QStringLiteral("Cerrar sesión"), this);
    toolbar->addWidget(m_status, 1);
    toolbar->addWidget(stop);
    layout->addLayout(toolbar);

    m_terminal = new VtTerminalWidget(this);
    layout->addWidget(m_terminal, 1);
    m_pty = new PtySession(this);
    connect(m_pty, &PtySession::outputReceived, m_terminal, &VtTerminalWidget::feed);
    connect(m_terminal, &VtTerminalWidget::inputReady, m_pty, &PtySession::write);
    connect(m_terminal, &VtTerminalWidget::terminalResized, m_pty, &PtySession::resize);
    connect(m_pty, &PtySession::processFinished, this, [this, stop](int exitCode) {
        m_status->setText(QStringLiteral("La terminal terminó · código %1").arg(exitCode));
        stop->setEnabled(false);
    });
    connect(m_pty, &PtySession::failed, this, [this, stop](const QString &message) {
        m_status->setText(message);
        stop->setEnabled(false);
    });
    connect(stop, &QPushButton::clicked, this, [this, stop] {
        m_pty->stop();
        m_status->setText(QStringLiteral("Terminal cerrada"));
        stop->setEnabled(false);
    });

#ifdef Q_OS_WIN
    const QString shell = qEnvironmentVariable("COMSPEC", QStringLiteral("cmd.exe"));
#else
    const QString shell = qEnvironmentVariable("SHELL", QStringLiteral("/bin/sh"));
#endif
    QString error;
    const QString cwd = QDir(workingDirectory).absolutePath();
    if (m_pty->start(shell, {}, cwd, 110, 32, &error)) {
        m_status->setText(QStringLiteral("Terminal del proyecto · %1").arg(cwd));
        m_terminal->setFocus(Qt::OtherFocusReason);
    } else {
        m_status->setText(QStringLiteral("No se pudo iniciar el shell: %1").arg(error));
        stop->setEnabled(false);
    }
}
