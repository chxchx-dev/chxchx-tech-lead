#pragma once

#include <QFrame>
#include <QStringList>
#include <QTemporaryDir>

class QHBoxLayout;
class QLabel;
class QPlainTextEdit;
class QPushButton;
class QToolButton;

// Message box of the visual chat, styled like modern IDE chats: rounded
// container, image attachments (button, paste or drag and drop), a toolbar for
// model/effort/permission pills and a round send/stop button.
class AgentChatComposer final : public QFrame {
    Q_OBJECT
public:
    explicit AgentChatComposer(QWidget *parent = nullptr);

    void addToolbarWidget(QWidget *widget);
    void setRunning(bool running);
    void setInputEnabled(bool enabled, const QString &reason = {});
    void setText(const QString &text);
    void setHint(const QString &hint);
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
    void resizeEvent(QResizeEvent *event) override;

private:
    bool pasteClipboardImage();
    void updateAttachments();
    void setFocused(bool focused);

    QPlainTextEdit *m_input = nullptr;
    QPushButton *m_send = nullptr;
    QToolButton *m_attach = nullptr;
    QHBoxLayout *m_attachmentRow = nullptr;
    QHBoxLayout *m_toolbar = nullptr;
    QLabel *m_hint = nullptr;
    QStringList m_attachments;
    QTemporaryDir m_pasted;
    bool m_running = false;
    int m_pastedCount = 0;
};
