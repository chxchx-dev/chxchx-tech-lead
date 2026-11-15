#pragma once

#include <QWidget>
#include <QStringList>

class QLabel;
class PtySession;
class VtTerminalWidget;

class WorkspaceTerminalWidget final : public QWidget {
    Q_OBJECT
public:
    explicit WorkspaceTerminalWidget(QString workingDirectory, QWidget *parent = nullptr,
        QStringList command = {}, QString sessionLabel = {});

private:
    QLabel *m_status = nullptr;
    PtySession *m_pty = nullptr;
    VtTerminalWidget *m_terminal = nullptr;
};
