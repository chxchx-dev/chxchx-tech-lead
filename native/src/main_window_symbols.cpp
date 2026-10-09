#include "main_window.hpp"
#include "symbol_index.hpp"

#include <ScintillaEditBase.h>
#include <Scintilla.h>
#include <ScintillaMessages.h>

#include <QDialog>
#include <QDialogButtonBox>
#include <QFileInfo>
#include <QLineEdit>
#include <QListWidget>
#include <QListWidgetItem>
#include <QPointer>
#include <QStatusBar>
#include <QVBoxLayout>

void MainWindow::showCurrentFileSymbols()
{
    auto *editor = currentEditor();
    if (editor == nullptr) {
        statusBar()->showMessage(QStringLiteral("Abre un archivo de código para navegar sus símbolos."), 4000);
        return;
    }

    const auto length = editor->send(SCI_GETLENGTH);
    QByteArray utf8(static_cast<qsizetype>(length + 1), '\0');
    editor->send(SCI_GETTEXT, static_cast<Scintilla::uptr_t>(utf8.size()),
        reinterpret_cast<Scintilla::sptr_t>(utf8.data()));
    const QString source = QString::fromUtf8(utf8.constData(), static_cast<qsizetype>(length));
    const QString path = editor->property("filePath").toString();
    const QList<SourceSymbol> symbols = SymbolIndex::scan(source, QFileInfo(path).suffix());

    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("Símbolos · %1").arg(QFileInfo(path).fileName()));
    dialog.resize(560, 520);
    auto *layout = new QVBoxLayout(&dialog);
    auto *filter = new QLineEdit(&dialog);
    filter->setPlaceholderText(QStringLiteral("Filtrar símbolos…"));
    auto *items = new QListWidget(&dialog);
    for (const SourceSymbol &symbol : symbols) {
        auto *item = new QListWidgetItem(QStringLiteral("%1  ·  %2  ·  línea %3")
            .arg(symbol.kind, symbol.name).arg(symbol.line), items);
        item->setData(Qt::UserRole, symbol.line);
        item->setData(Qt::UserRole + 1, symbol.name);
    }
    if (items->count() == 0) {
        auto *item = new QListWidgetItem(QStringLiteral(
            "No se encontraron declaraciones compatibles con este lenguaje."), items);
        item->setFlags(Qt::NoItemFlags);
    } else {
        items->setCurrentRow(0);
    }
    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Close, &dialog);
    layout->addWidget(filter);
    layout->addWidget(items, 1);
    layout->addWidget(buttons);
    connect(filter, &QLineEdit::textChanged, &dialog, [items](const QString &text) {
        for (int index = 0; index < items->count(); ++index) {
            QListWidgetItem *item = items->item(index);
            item->setHidden(!item->data(Qt::UserRole + 1).toString().contains(
                text, Qt::CaseInsensitive) && !item->text().contains(text, Qt::CaseInsensitive));
        }
    });
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    connect(items, &QListWidget::itemActivated, &dialog, [&dialog](QListWidgetItem *) { dialog.accept(); });
    filter->setFocus();
    QPointer<ScintillaEditBase> target(editor);
    if (dialog.exec() != QDialog::Accepted || target.isNull() || items->currentItem() == nullptr) return;
    const int line = items->currentItem()->data(Qt::UserRole).toInt();
    if (line <= 0) return;
    target->send(SCI_GOTOLINE, line - 1);
    target->setFocus(Qt::OtherFocusReason);
}
