#pragma once

#include <QStringList>
#include <QTemporaryDir>
#include <QWidget>

class QLabel;
class QPlainTextEdit;
class QPushButton;
class QToolButton;

// Message box of the visual chat: text, image attachments (button, paste or
// drag and drop) and the send/stop button.
class AgentChatComposer final : public QWidget {
    Q_OBJECT
public:
    explicit AgentChatComposer(QWidget *parent = nullptr);

    void setRunning(bool running);
    void setInputEnabled(bool enabled, const QString &reason = {});
    void focusInput();
    void clear();
    QStringList attachments() const { return m_attachments; }
    // Adds image files; returns how many were accepted. Public for tests.
    int addAttachments(const QStringList &paths);

signals:
    void sendRequested(const QString &text, const QStringList &images);
    void stopRequested();

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    bool pasteClipboardImage();
    void updateAttachmentLabel();

    QPlainTextEdit *m_input = nullptr;
    QPushButton *m_send = nullptr;
    QToolButton *m_attach = nullptr;
    QLabel *m_attachmentLabel = nullptr;
    QToolButton *m_clearAttachments = nullptr;
    QStringList m_attachments;
    QTemporaryDir m_pasted;
    bool m_running = false;
    int m_pastedCount = 0;
};
