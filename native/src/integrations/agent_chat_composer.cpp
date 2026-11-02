#include "agent_chat_composer.hpp"
#include "chat_pill_picker.hpp"

#include <QApplication>
#include <QClipboard>
#include <QDragEnterEvent>
#include <QDropEvent>
#include <QFileDialog>
#include <QFileInfo>
#include <QHBoxLayout>
#include <QImage>
#include <QKeyEvent>
#include <QLabel>
#include <QMimeData>
#include <QPalette>
#include <QPixmap>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QResizeEvent>
#include <QStyle>
#include <QToolButton>
#include <QVBoxLayout>

namespace {

constexpr int MaxAttachments = 6;
const QString IdleHint = QStringLiteral("Ctrl+Enter para enviar");

bool isImageFile(const QString &path)
{
    static const QStringList suffixes = {QStringLiteral("png"), QStringLiteral("jpg"), QStringLiteral("jpeg"),
        QStringLiteral("gif"), QStringLiteral("webp")};
    const QFileInfo info(path);
    return info.isFile() && suffixes.contains(info.suffix().toLower());
}

void repolish(QWidget *widget)
{
    widget->style()->unpolish(widget);
    widget->style()->polish(widget);
}

} // namespace

AgentChatComposer::AgentChatComposer(QWidget *parent)
    : QFrame(parent)
{
    setObjectName(QStringLiteral("composer"));
    auto *column = new QVBoxLayout(this);
    column->setContentsMargins(12, 10, 10, 8);
    column->setSpacing(6);

    m_attachmentRow = new QHBoxLayout;
    m_attachmentRow->setSpacing(6);
    column->addLayout(m_attachmentRow);

    m_input = new QPlainTextEdit(this);
    m_input->setObjectName(QStringLiteral("composerInput"));
    m_input->setPlaceholderText(QStringLiteral("Pide un cambio, una explicación o una revisión…"));
    m_input->setFixedHeight(64);
    m_input->setAcceptDrops(true);
    m_input->installEventFilter(this);
    // The chat uses its own dark surfaces, so hints must not follow a light system theme.
    QPalette hints = m_input->palette();
    hints.setColor(QPalette::PlaceholderText, QColor(0x8d, 0x96, 0xa8));
    m_input->setPalette(hints);
    column->addWidget(m_input);

    m_toolbar = new QHBoxLayout;
    m_toolbar->setSpacing(6);
    m_attach = new QToolButton(this);
    m_attach->setObjectName(QStringLiteral("composerIcon"));
    m_attach->setText(QStringLiteral("📎"));
    m_attach->setCursor(Qt::PointingHandCursor);
    m_attach->setToolTip(QStringLiteral("Adjuntar imágenes. También puedes pegarlas (Ctrl+V) o arrastrarlas aquí."));
    m_hint = new QLabel(IdleHint, this);
    m_hint->setObjectName(QStringLiteral("composerHint"));
    m_hint->setMinimumWidth(1);
    m_send = new QPushButton(QStringLiteral("↑"), this);
    m_send->setObjectName(QStringLiteral("sendButton"));
    m_send->setCursor(Qt::PointingHandCursor);
    m_send->setToolTip(QStringLiteral("Enviar (Ctrl+Enter)"));
    m_toolbar->addWidget(m_attach);
    m_toolbar->addStretch(1);
    m_toolbar->addWidget(m_hint);
    m_toolbar->addWidget(m_send);
    column->addLayout(m_toolbar);

    connect(m_send, &QPushButton::clicked, this, [this] {
        if (m_running) {
            emit stopRequested();
            return;
        }
        const QString text = m_input->toPlainText().trimmed();
        if (text.isEmpty() && m_attachments.isEmpty()) return;
        emit sendRequested(text.isEmpty() ? QStringLiteral("Revisa las imágenes adjuntas.") : text, m_attachments);
    });
    connect(m_attach, &QToolButton::clicked, this, [this] {
        addAttachments(QFileDialog::getOpenFileNames(this, QStringLiteral("Adjuntar imágenes"), {},
            QStringLiteral("Imágenes (*.png *.jpg *.jpeg *.gif *.webp)")));
    });
    updateAttachments();
}

void AgentChatComposer::addToolbarWidget(QWidget *widget)
{
    // Pills sit before the trailing stretch, hint and send button.
    m_toolbar->insertWidget(m_toolbar->count() - 3, widget);
}

void AgentChatComposer::setRunning(bool running)
{
    m_running = running;
    m_send->setText(running ? QStringLiteral("■") : QStringLiteral("↑"));
    m_send->setToolTip(running ? QStringLiteral("Detener al agente") : QStringLiteral("Enviar (Ctrl+Enter)"));
    m_send->setProperty("running", running);
    repolish(m_send);
    m_attach->setEnabled(!running && m_input->isEnabled());
    m_hint->setText(running ? QStringLiteral("Puedes detenerlo con ■") : IdleHint);
}

void AgentChatComposer::setInputEnabled(bool enabled, const QString &reason)
{
    m_input->setEnabled(enabled);
    m_send->setEnabled(enabled);
    m_attach->setEnabled(enabled && !m_running);
    if (!reason.isEmpty()) m_input->setPlaceholderText(reason);
}

void AgentChatComposer::setText(const QString &text)
{
    m_input->setPlainText(text);
    m_input->moveCursor(QTextCursor::End);
    focusInput();
}

void AgentChatComposer::setHint(const QString &hint)
{
    m_hint->setText(hint.isEmpty() ? IdleHint : hint);
}

void AgentChatComposer::focusInput()
{
    m_input->setFocus(Qt::OtherFocusReason);
}

void AgentChatComposer::clear()
{
    m_input->clear();
    m_attachments.clear();
    updateAttachments();
}

int AgentChatComposer::addAttachments(const QStringList &paths)
{
    int added = 0;
    for (const QString &path : paths) {
        const QString absolute = QFileInfo(path).absoluteFilePath();
        if (m_attachments.size() >= MaxAttachments || !isImageFile(path) || m_attachments.contains(absolute)) continue;
        m_attachments << absolute;
        ++added;
    }
    updateAttachments();
    return added;
}

bool AgentChatComposer::eventFilter(QObject *watched, QEvent *event)
{
    if (watched != m_input) return QFrame::eventFilter(watched, event);
    switch (event->type()) {
    case QEvent::FocusIn: setFocused(true); break;
    case QEvent::FocusOut: setFocused(false); break;
    case QEvent::KeyPress: {
        auto *key = static_cast<QKeyEvent *>(event);
        if ((key->key() == Qt::Key_Return || key->key() == Qt::Key_Enter)
            && key->modifiers().testFlag(Qt::ControlModifier)) {
            m_send->click();
            return true;
        }
        if (key->matches(QKeySequence::Paste) && pasteClipboardImage()) return true;
        break;
    }
    case QEvent::DragEnter: {
        auto *drag = static_cast<QDragEnterEvent *>(event);
        if (drag->mimeData()->hasUrls()) {
            drag->acceptProposedAction();
            return true;
        }
        break;
    }
    case QEvent::Drop: {
        auto *drop = static_cast<QDropEvent *>(event);
        QStringList files;
        for (const QUrl &url : drop->mimeData()->urls()) {
            if (url.isLocalFile()) files << url.toLocalFile();
        }
        if (addAttachments(files) > 0) {
            drop->acceptProposedAction();
            return true;
        }
        break;
    }
    default: break;
    }
    return QFrame::eventFilter(watched, event);
}

bool AgentChatComposer::pasteClipboardImage()
{
    const QMimeData *data = QApplication::clipboard()->mimeData();
    if (data == nullptr || !data->hasImage() || !m_pasted.isValid()) return false;
    const QImage image = qvariant_cast<QImage>(data->imageData());
    if (image.isNull()) return false;
    const QString path = m_pasted.filePath(QStringLiteral("captura-%1.png").arg(++m_pastedCount));
    if (!image.save(path, "PNG")) return false;
    return addAttachments({path}) > 0;
}

void AgentChatComposer::updateAttachments()
{
    while (QLayoutItem *item = m_attachmentRow->takeAt(0)) {
        delete item->widget();
        delete item;
    }
    for (const QString &path : m_attachments) {
        auto *chip = new QFrame(this);
        chip->setObjectName(QStringLiteral("attachmentChip"));
        auto *row = new QHBoxLayout(chip);
        row->setContentsMargins(4, 2, 2, 2);
        auto *thumb = new QLabel(chip);
        thumb->setPixmap(QPixmap(path).scaled(28, 28, Qt::KeepAspectRatioByExpanding, Qt::SmoothTransformation));
        auto *name = new QLabel(QFileInfo(path).fileName(), chip);
        auto *remove = new QToolButton(chip);
        remove->setObjectName(QStringLiteral("chatGhost"));
        remove->setText(QStringLiteral("✕"));
        remove->setToolTip(QStringLiteral("Quitar"));
        connect(remove, &QToolButton::clicked, this, [this, path] {
            m_attachments.removeAll(path);
            updateAttachments();
        });
        row->addWidget(thumb);
        row->addWidget(name);
        row->addWidget(remove);
        m_attachmentRow->addWidget(chip);
    }
    m_attachmentRow->addStretch(1);
}

void AgentChatComposer::resizeEvent(QResizeEvent *event)
{
    QFrame::resizeEvent(event);
    // Narrow panels (split tabs) drop secondary text instead of forcing width.
    const int width = event->size().width();
    m_hint->setVisible(width >= 700);
    for (ChatPillPicker *pill : findChildren<ChatPillPicker *>()) pill->setCompact(width < 600);
}

void AgentChatComposer::setFocused(bool focused)
{
    setProperty("focused", focused);
    repolish(this);
}
