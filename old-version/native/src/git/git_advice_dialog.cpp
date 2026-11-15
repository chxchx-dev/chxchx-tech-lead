#include "git_advice_dialog.hpp"

#include "../studio_icons.hpp"
#include "../studio_theme.hpp"

#include <QApplication>
#include <QClipboard>
#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QTextBrowser>
#include <QVBoxLayout>

GitAdviceDialog::GitAdviceDialog(const QString &title, const QString &markdown, const QString &agentLabel, QWidget *parent)
    : QDialog(parent), m_markdown(markdown)
{
    setWindowTitle(title);
    setObjectName(QStringLiteral("gitAdviceDialog"));
    resize(560, 520);
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(16, 14, 16, 14);
    layout->setSpacing(10);
    auto *header = new QHBoxLayout;
    auto *icon = new QLabel(this);
    icon->setPixmap(renderStudioIcon(QStringLiteral("studio-advice"), ThemeManager::current().accent, 26));
    auto *heading = new QLabel(QStringLiteral("<b style='font-size:15px'>%1</b><br><span style='color:%2'>Consejo de %3. Revísalo antes de ejecutar nada: el asistente solo aconseja.</span>")
        .arg(title.toHtmlEscaped(), ThemeManager::current().textMuted.name(), agentLabel.toHtmlEscaped()), this);
    heading->setWordWrap(true);
    header->addWidget(icon, 0, Qt::AlignTop);
    header->addWidget(heading, 1);
    layout->addLayout(header);
    m_view = new QTextBrowser(this);
    m_view->setOpenExternalLinks(true);
    m_view->setMarkdown(markdown);
    layout->addWidget(m_view, 1);
    auto *buttons = new QHBoxLayout;
    auto *copy = new QPushButton(studioIcon(QStringLiteral("edit-copy"), QStyle::SP_FileIcon), QStringLiteral("Copiar el consejo"), this);
    auto *close = new QPushButton(QStringLiteral("Cerrar"), this);
    close->setDefault(true);
    buttons->addStretch(1);
    buttons->addWidget(copy);
    buttons->addWidget(close);
    layout->addLayout(buttons);
    connect(copy, &QPushButton::clicked, this, [this] { QApplication::clipboard()->setText(m_markdown); });
    connect(close, &QPushButton::clicked, this, &QDialog::accept);
}

QString GitAdviceDialog::renderedText() const
{
    return m_view->toPlainText();
}
