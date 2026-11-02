#include "agent_chat_composer.hpp"

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
#include <QPlainTextEdit>
#include <QPushButton>
#include <QToolButton>
#include <QVBoxLayout>

namespace {

constexpr int MaxAttachments = 6;

bool isImageFile(const QString &path)
{
    static const QStringList suffixes = {QStringLiteral("png"), QStringLiteral("jpg"), QStringLiteral("jpeg"),
        QStringLiteral("gif"), QStringLiteral("webp")};
    const QFileInfo info(path);
    return info.isFile() && suffixes.contains(info.suffix().toLower());
}

} // namespace

AgentChatComposer::AgentChatComposer(QWidget *parent)
    : QWidget(parent)
{
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(4);

    auto *attachments = new QHBoxLayout;
    m_attachmentLabel = new QLabel(this);
    m_attachmentLabel->setStyleSheet(QStringLiteral("color: #a8b3c4;"));
    m_clearAttachments = new QToolButton(this);
    m_clearAttachments->setText(QStringLiteral("✕"));
    m_clearAttachments->setToolTip(QStringLiteral("Quitar imágenes adjuntas"));
    attachments->addWidget(m_attachmentLabel, 1);
    attachments->addWidget(m_clearAttachments);
    layout->addLayout(attachments);

    auto *row = new QHBoxLayout;
    m_attach = new QToolButton(this);
    m_attach->setText(QStringLiteral("📎"));
    m_attach->setToolTip(QStringLiteral("Adjuntar imágenes. También puedes pegarlas (Ctrl+V) o arrastrarlas aquí."));
    m_attach->setMinimumSize(36, 36);
    m_input = new QPlainTextEdit(this);
    m_input->setPlaceholderText(QStringLiteral("Mensaje para el agente… (Ctrl+Enter para enviar)"));
    m_input->setMaximumHeight(110);
    m_input->setAcceptDrops(true);
    m_input->installEventFilter(this);
    // The chat uses its own dark surfaces, so hints must not follow a light system theme.
    QPalette hints = m_input->palette();
    hints.setColor(QPalette::PlaceholderText, QColor(0x8d, 0x96, 0xa8));
    m_input->setPalette(hints);
    m_send = new QPushButton(QStringLiteral("Enviar"), this);
    m_send->setMinimumHeight(36);
    row->addWidget(m_attach, 0, Qt::AlignBottom);
    row->addWidget(m_input, 1);
    row->addWidget(m_send, 0, Qt::AlignBottom);
    layout->addLayout(row);

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
        const QStringList files = QFileDialog::getOpenFileNames(this, QStringLiteral("Adjuntar imágenes"), {},
            QStringLiteral("Imágenes (*.png *.jpg *.jpeg *.gif *.webp)"));
        addAttachments(files);
    });
    connect(m_clearAttachments, &QToolButton::clicked, this, [this] {
        m_attachments.clear();
        updateAttachmentLabel();
    });
    updateAttachmentLabel();
}

void AgentChatComposer::setRunning(bool running)
{
    m_running = running;
    m_send->setText(running ? QStringLiteral("Detener") : QStringLiteral("Enviar"));
    m_attach->setEnabled(!running && m_input->isEnabled());
}

void AgentChatComposer::setInputEnabled(bool enabled, const QString &reason)
{
    m_input->setEnabled(enabled);
    m_send->setEnabled(enabled);
    m_attach->setEnabled(enabled && !m_running);
    if (!reason.isEmpty()) m_input->setPlaceholderText(reason);
}

void AgentChatComposer::focusInput()
{
    m_input->setFocus(Qt::OtherFocusReason);
}

void AgentChatComposer::clear()
{
    m_input->clear();
    m_attachments.clear();
    updateAttachmentLabel();
}

int AgentChatComposer::addAttachments(const QStringList &paths)
{
    int added = 0;
    for (const QString &path : paths) {
        if (m_attachments.size() >= MaxAttachments || !isImageFile(path) || m_attachments.contains(path)) continue;
        m_attachments << QFileInfo(path).absoluteFilePath();
        ++added;
    }
    updateAttachmentLabel();
    return added;
}

bool AgentChatComposer::eventFilter(QObject *watched, QEvent *event)
{
    if (watched != m_input) return QWidget::eventFilter(watched, event);
    if (event->type() == QEvent::KeyPress) {
        auto *key = static_cast<QKeyEvent *>(event);
        if ((key->key() == Qt::Key_Return || key->key() == Qt::Key_Enter)
            && key->modifiers().testFlag(Qt::ControlModifier)) {
            m_send->click();
            return true;
        }
        if (key->matches(QKeySequence::Paste) && pasteClipboardImage()) return true;
    } else if (event->type() == QEvent::DragEnter) {
        auto *drag = static_cast<QDragEnterEvent *>(event);
        if (drag->mimeData()->hasUrls()) {
            drag->acceptProposedAction();
            return true;
        }
    } else if (event->type() == QEvent::Drop) {
        auto *drop = static_cast<QDropEvent *>(event);
        QStringList files;
        for (const QUrl &url : drop->mimeData()->urls()) {
            if (url.isLocalFile()) files << url.toLocalFile();
        }
        if (addAttachments(files) > 0) {
            drop->acceptProposedAction();
            return true;
        }
    }
    return QWidget::eventFilter(watched, event);
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

void AgentChatComposer::updateAttachmentLabel()
{
    const bool any = !m_attachments.isEmpty();
    m_attachmentLabel->setVisible(any);
    m_clearAttachments->setVisible(any);
    if (!any) return;
    QStringList names;
    for (const QString &path : m_attachments) names << QFileInfo(path).fileName();
    m_attachmentLabel->setText(QStringLiteral("🖼 %1 imagen(es): %2").arg(m_attachments.size()).arg(names.join(QStringLiteral(", "))));
}
