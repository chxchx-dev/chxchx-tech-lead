#include "main_window.hpp"
#include "main_window_areas.hpp"
#include "project_file_index.hpp"
#include "studio_icons.hpp"

#include <QAction>
#include <QApplication>
#include <QComboBox>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDir>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QHash>
#include <QJsonArray>
#include <QJsonObject>
#include <QKeySequenceEdit>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QList>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QPushButton>
#include <QSettings>
#include <QTimer>
#include <QToolBar>
#include <QTabWidget>
#include <QVBoxLayout>

#include <functional>
#include <utility>

void MainWindow::buildActions()
{
    QSettings settings;
    m_wordWrapEnabled = settings.value(QStringLiteral("editor/wordWrap"), true).toBool();
    auto *fileMenu = menuBar()->addMenu(QStringLiteral("Archivo"));
    auto *editMenu = menuBar()->addMenu(QStringLiteral("Edición"));
    m_viewMenu = menuBar()->addMenu(QStringLiteral("Ver"));
    auto *navigateMenu = menuBar()->addMenu(QStringLiteral("Navegar"));
    auto *recentFiles = navigateMenu->addMenu(QStringLiteral("Archivos recientes"));
    connect(recentFiles, &QMenu::aboutToShow, this, [this, recentFiles] {
        recentFiles->clear();
        const QStringList paths = recentProjectFiles();
        if (paths.isEmpty()) {
            QAction *empty = recentFiles->addAction(QStringLiteral("Sin archivos recientes"));
            empty->setEnabled(false);
            return;
        }
        for (const QString &path : paths) {
            QAction *action = recentFiles->addAction(QDir(m_projectPath).relativeFilePath(path));
            action->setToolTip(path);
            connect(action, &QAction::triggered, this, [this, path] { openPath(path); });
        }
    });
    auto *toolsMenu = menuBar()->addMenu(QStringLiteral("Herramientas"));
    auto *helpMenu = menuBar()->addMenu(QStringLiteral("Ayuda"));

    m_quickToolbar = addToolBar(QStringLiteral("Acceso rápido"));
    auto *toolbar = m_quickToolbar;
    toolbar->setMovable(false);
    toolbar->setToolButtonStyle(Qt::ToolButtonIconOnly);
    toolbar->setIconSize(QSize(20, 20));

    const auto addAction = [this, &settings, toolbar](QMenu *menu, const QString &id,
        const QString &label, const QKeySequence &defaultShortcut,
        const std::function<void()> &callback, const QString &iconName,
        QStyle::StandardPixmap fallback, bool showOnToolbar = true) {
        auto *action = new QAction(label, this);
        const QString key = QStringLiteral("shortcuts/") + id;
        const QString defaultText = defaultShortcut.toString(QKeySequence::PortableText);
        const QString configured = settings.value(key, defaultText).toString();
        action->setShortcut(QKeySequence::fromString(configured, QKeySequence::PortableText));
        action->setIcon(studioIcon(iconName, fallback));
        action->setProperty("shortcutId", id);
        action->setProperty("defaultShortcut", defaultText);
        action->setStatusTip(label);
        action->setToolTip(action->shortcut().isEmpty()
            ? label : QStringLiteral("%1 (%2)").arg(label, action->shortcut().toString(QKeySequence::NativeText)));
        connect(action, &QAction::triggered, this, callback);
        menu->addAction(action);
        if (!id.isEmpty()) {
            m_shortcutActions.insert(id, action);
            m_shortcutOrder.append(id);
        }
        if (showOnToolbar) {
            toolbar->addAction(action);
        }
        return action;
    };

    addAction(fileMenu, QStringLiteral("file.open"), QStringLiteral("Abrir archivo…"),
        QKeySequence::Open, [this] { openFile(); }, QStringLiteral("document-open"), QStyle::SP_DialogOpenButton);
    addAction(fileMenu, QStringLiteral("file.save"), QStringLiteral("Guardar"),
        QKeySequence::Save, [this] { saveFile(); }, QStringLiteral("document-save"), QStyle::SP_DialogSaveButton);
    addAction(fileMenu, QStringLiteral("file.saveAs"), QStringLiteral("Guardar como…"),
        QKeySequence::SaveAs, [this] { saveFileAs(); }, QStringLiteral("document-save-as"), QStyle::SP_FileIcon);
    addAction(fileMenu, QStringLiteral("file.closeTab"), QStringLiteral("Cerrar pestaña"),
        QKeySequence::Close, [this] {
            QTabWidget *tabs = activeEditorTabs();
            closeEditorTab(tabs, tabs->currentIndex());
        },
        QStringLiteral("tab-close"), QStyle::SP_DialogCloseButton, false);

    addAction(editMenu, QStringLiteral("edit.find"), QStringLiteral("Buscar en archivo…"),
        QKeySequence::Find, [this] { findInCurrentFile(); }, QStringLiteral("edit-find"),
        QStyle::SP_FileDialogContentsView);
    editMenu->addSeparator();
    addAction(editMenu, QStringLiteral("edit.undo"), QStringLiteral("Deshacer"),
        QKeySequence::Undo, [this] { performEditAction(QStringLiteral("edit.undo")); },
        QStringLiteral("edit-undo"), QStyle::SP_ArrowBack, false);
    addAction(editMenu, QStringLiteral("edit.redo"), QStringLiteral("Rehacer"),
        QKeySequence::Redo, [this] { performEditAction(QStringLiteral("edit.redo")); },
        QStringLiteral("edit-redo"), QStyle::SP_ArrowForward, false);
    editMenu->addSeparator();
    addAction(editMenu, QStringLiteral("edit.cut"), QStringLiteral("Cortar"),
        QKeySequence::Cut, [this] { performEditAction(QStringLiteral("edit.cut")); },
        QStringLiteral("edit-cut"), QStyle::SP_FileDialogDetailedView, false);
    addAction(editMenu, QStringLiteral("edit.copy"), QStringLiteral("Copiar"),
        QKeySequence::Copy, [this] { performEditAction(QStringLiteral("edit.copy")); },
        QStringLiteral("edit-copy"), QStyle::SP_FileDialogContentsView, false);
    addAction(editMenu, QStringLiteral("edit.paste"), QStringLiteral("Pegar"),
        QKeySequence::Paste, [this] { performEditAction(QStringLiteral("edit.paste")); },
        QStringLiteral("edit-paste"), QStyle::SP_DialogOpenButton, false);
    editMenu->addSeparator();
    addAction(editMenu, QStringLiteral("edit.selectAll"), QStringLiteral("Seleccionar todo"),
        QKeySequence::SelectAll, [this] { performEditAction(QStringLiteral("edit.selectAll")); },
        QStringLiteral("edit-select-all"), QStyle::SP_DialogApplyButton, false);
    editMenu->addSeparator();
    auto *shortcuts = new QAction(QStringLiteral("Configurar atajos…"), this);
    connect(shortcuts, &QAction::triggered, this, &MainWindow::configureShortcuts);
    editMenu->addAction(shortcuts);

    addAction(m_viewMenu, QStringLiteral("view.refresh"), QStringLiteral("Actualizar vista"),
        QKeySequence(QStringLiteral("F5")), [this] { refreshArea(); }, QStringLiteral("view-refresh"),
        QStyle::SP_BrowserReload);
    auto *wordWrap = addAction(m_viewMenu, QStringLiteral("view.wordWrap"),
        QStringLiteral("Ajuste de línea"), QKeySequence(QStringLiteral("Alt+Z")),
        [this] { setWordWrapEnabled(!m_wordWrapEnabled); }, QStringLiteral("format-justify-fill"),
        QStyle::SP_TitleBarUnshadeButton, false);
    wordWrap->setCheckable(true);
    wordWrap->setChecked(m_wordWrapEnabled);
    navigateMenu->addAction(QStringLiteral("Paleta de comandos…"),
        QKeySequence(QStringLiteral("Ctrl+P")), this, &MainWindow::openCommandPalette);
    auto *paletteAction = navigateMenu->actions().constLast();
    paletteAction->setIcon(studioIcon(QStringLiteral("system-search"), QStyle::SP_CommandLink));
    paletteAction->setProperty("shortcutId", QStringLiteral("navigate.palette"));
    paletteAction->setProperty("defaultShortcut", QStringLiteral("Ctrl+P"));
    const QString paletteShortcut = settings.value(QStringLiteral("shortcuts/navigate.palette"),
        QStringLiteral("Ctrl+P")).toString();
    paletteAction->setShortcut(QKeySequence::fromString(paletteShortcut, QKeySequence::PortableText));
    paletteAction->setStatusTip(QStringLiteral("Paleta de comandos"));
    paletteAction->setToolTip(QStringLiteral("Paleta de comandos (%1)")
        .arg(paletteAction->shortcut().toString(QKeySequence::NativeText)));
    m_shortcutActions.insert(QStringLiteral("navigate.palette"), paletteAction);
    m_shortcutOrder.append(QStringLiteral("navigate.palette"));
    addAction(navigateMenu, QStringLiteral("navigate.projectSearch"),
        QStringLiteral("Buscar archivo del proyecto…"), QKeySequence(QStringLiteral("Ctrl+Shift+F")),
        [this] { searchProjectFiles(); }, QStringLiteral("system-search"),
        QStyle::SP_FileDialogDetailedView, false);
    addAction(navigateMenu, QStringLiteral("navigate.symbols"),
        QStringLiteral("Símbolos del archivo actual…"), QKeySequence(QStringLiteral("Ctrl+Shift+O")),
        [this] { showCurrentFileSymbols(); }, QStringLiteral("code-context"),
        QStyle::SP_FileDialogListView, false);

    addAction(toolsMenu, QStringLiteral("tools.terminal"), QStringLiteral("Nueva terminal integrada"),
        QKeySequence(QStringLiteral("Ctrl+Shift+T")), [this] { openNewWorkspaceTerminal(); },
        QStringLiteral("utilities-terminal"), QStyle::SP_ComputerIcon, false);
    addAction(toolsMenu, QStringLiteral("tools.gitDiff"), QStringLiteral("Diff Git del archivo actual"),
        QKeySequence(), [this] { showCurrentFileDiff(); }, QStringLiteral("vcs-diff"),
        QStyle::SP_FileDialogDetailedView, false);
    helpMenu->addAction(QStringLiteral("Guía rápida"), this, [this] {
        for (int row = 0; row < m_areaList->count(); ++row) {
            if (m_areaList->item(row)->data(Qt::UserRole).toString() == QStringLiteral("guide")) {
                m_areaList->setCurrentRow(row);
                break;
            }
        }
    });
    helpMenu->addAction(QStringLiteral("Acerca de ChxChx Studio"), this, [this] {
        QMessageBox::about(this, QStringLiteral("Acerca de ChxChx Studio"),
            QStringLiteral("ChxChx Studio\nControl plane local para proyectos de desarrollo asistido."));
    });

    toolbar->addSeparator();
    toolbar->addAction(m_shortcutActions.value(QStringLiteral("tools.terminal")));
    toolbar->addAction(paletteAction);

    connect(qApp, &QApplication::focusChanged, this, [this](QWidget *, QWidget *now) {
        if (now == nullptr || qobject_cast<QMenu *>(now) != nullptr
            || qobject_cast<QMenuBar *>(now) != nullptr) return;
        m_editTargetWidget = now;
        updateEditActionState();
    });
    updateEditActionState();
}

void MainWindow::searchProjectFiles()
{
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("Buscar en el proyecto"));
    dialog.resize(720, 520);
    auto *layout = new QVBoxLayout(&dialog);
    auto *mode = new QComboBox(&dialog);
    mode->addItem(QStringLiteral("Nombres y rutas"));
    mode->addItem(QStringLiteral("Contenido de archivos"));
    auto *query = new QLineEdit(&dialog);
    query->setPlaceholderText(QStringLiteral("Buscar por nombre o ruta relativa…"));
    auto *summary = new QLabel(&dialog);
    auto *results = new QListWidget(&dialog);
    auto *buttons = new QHBoxLayout();
    auto *reindex = new QPushButton(QStringLiteral("Actualizar índice"), &dialog);
    auto *close = new QPushButton(QStringLiteral("Cerrar"), &dialog);
    buttons->addWidget(reindex);
    buttons->addStretch(1);
    buttons->addWidget(close);
    layout->addWidget(mode);
    layout->addWidget(query);
    layout->addWidget(summary);
    layout->addWidget(results, 1);
    layout->addLayout(buttons);

    auto *contentSearchDelay = new QTimer(&dialog);
    contentSearchDelay->setSingleShot(true);
    contentSearchDelay->setInterval(250);
    const auto updateResults = [this, mode, query, summary, results, reindex, contentSearchDelay] {
        results->clear();
        if (!m_fileIndex->isReady()) {
            summary->setText(QStringLiteral("Indexando archivos en segundo plano…"));
            reindex->setEnabled(false);
            return;
        }
        reindex->setEnabled(true);
        const QString term = query->text().trimmed();
        if (term.isEmpty()) {
            summary->setText(QStringLiteral("Índice listo: %1 archivos. Escribe para buscar.")
                .arg(m_fileIndex->fileCount()));
            return;
        }
        if (mode->currentIndex() == 1) {
            summary->setText(QStringLiteral("Buscando contenido en segundo plano…"));
            contentSearchDelay->start();
            return;
        }
        const QStringList paths = m_fileIndex->search(term);
        summary->setText(QStringLiteral("Coincidencias mostradas: %1 (máximo 250).")
            .arg(paths.size()));
        for (const QString &path : paths) {
            auto *item = new QListWidgetItem(QDir(m_projectPath).relativeFilePath(path), results);
            item->setData(Qt::UserRole, path);
            item->setIcon(studioIcon(QStringLiteral("text-x-generic"), QStyle::SP_FileIcon));
        }
        if (results->count() > 0) results->setCurrentRow(0);
    };
    connect(query, &QLineEdit::textChanged, &dialog, [updateResults] { updateResults(); });
    connect(mode, qOverload<int>(&QComboBox::currentIndexChanged), &dialog,
        [mode, query, updateResults] {
        query->setPlaceholderText(mode->currentIndex() == 1
            ? QStringLiteral("Buscar texto en el contenido…")
            : QStringLiteral("Buscar por nombre o ruta relativa…"));
        updateResults();
    });
    connect(contentSearchDelay, &QTimer::timeout, &dialog, [this, mode, query] {
        if (mode->currentIndex() == 1) m_fileIndex->searchContent(query->text());
    });
    connect(m_fileIndex, &ProjectFileIndex::contentSearchFinished, &dialog,
        [this, mode, query, summary, results](const QString &searched,
            const QJsonArray &matches, bool truncated) {
            if (mode->currentIndex() != 1 || query->text().trimmed() != searched) return;
            results->clear();
            summary->setText(QStringLiteral("Coincidencias en contenido: %1%2")
                .arg(matches.size())
                .arg(truncated ? QStringLiteral(" · resultado limitado por tamaño/cantidad") : QString()));
            for (const auto &value : matches) {
                const QJsonObject match = value.toObject();
                const QString path = match.value(QStringLiteral("path")).toString();
                const int line = match.value(QStringLiteral("line")).toInt();
                const QString relative = QDir(m_projectPath).relativeFilePath(path);
                auto *item = new QListWidgetItem(QStringLiteral("%1:%2  %3")
                    .arg(relative).arg(line).arg(match.value(QStringLiteral("excerpt")).toString()), results);
                item->setData(Qt::UserRole, path);
                item->setData(Qt::UserRole + 1, line);
                item->setIcon(studioIcon(QStringLiteral("text-x-generic"), QStyle::SP_FileIcon));
            }
            if (results->count() > 0) results->setCurrentRow(0);
        });
    connect(m_fileIndex, &ProjectFileIndex::indexChanged, &dialog,
        [updateResults](int, bool) { updateResults(); });
    connect(reindex, &QPushButton::clicked, m_fileIndex, &ProjectFileIndex::rebuild);
    connect(close, &QPushButton::clicked, &dialog, &QDialog::reject);
    connect(results, &QListWidget::itemActivated, &dialog, &QDialog::accept);
    connect(query, &QLineEdit::returnPressed, &dialog, [&dialog, results] {
        if (results->count() > 0) dialog.accept();
    });
    updateResults();
    query->setFocus();
    if (dialog.exec() == QDialog::Accepted && results->currentItem() != nullptr) {
        const auto *selected = results->currentItem();
        openPath(selected->data(Qt::UserRole).toString(), selected->data(Qt::UserRole + 1).toInt());
    }
}

void MainWindow::configureShortcuts()
{
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("Atajos de teclado"));
    dialog.setMinimumWidth(480);
    auto *layout = new QVBoxLayout(&dialog);
    auto *description = new QLabel(
        QStringLiteral("Selecciona un campo y pulsa la combinación que quieras asignar."), &dialog);
    layout->addWidget(description);

    auto *form = new QFormLayout();
    QList<QPair<QAction *, QKeySequenceEdit *>> editors;
    for (const QString &id : m_shortcutOrder) {
        QAction *action = m_shortcutActions.value(id);
        auto *editor = new QKeySequenceEdit(action->shortcut(), &dialog);
        editor->setObjectName(id);
        form->addRow(action->text(), editor);
        editors.append(qMakePair(action, editor));
    }
    layout->addLayout(form);

    auto *buttons = new QDialogButtonBox(&dialog);
    buttons->addButton(QStringLiteral("Guardar"), QDialogButtonBox::AcceptRole);
    buttons->addButton(QStringLiteral("Cancelar"), QDialogButtonBox::RejectRole);
    auto *reset = buttons->addButton(QStringLiteral("Restablecer"), QDialogButtonBox::ResetRole);
    connect(reset, &QPushButton::clicked, &dialog, [&editors] {
        for (const auto &entry : editors) {
            entry.second->setKeySequence(QKeySequence::fromString(
                entry.first->property("defaultShortcut").toString(), QKeySequence::PortableText));
        }
    });
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, [&dialog, &editors] {
        QHash<QString, QString> assigned;
        for (const auto &entry : editors) {
            const QString shortcut = entry.second->keySequence().toString(QKeySequence::PortableText);
            if (shortcut.isEmpty()) continue;
            if (assigned.contains(shortcut)) {
                QMessageBox::warning(&dialog, QStringLiteral("Atajo repetido"),
                    QStringLiteral("%1 y %2 usan el mismo atajo: %3")
                        .arg(assigned.value(shortcut), entry.first->text(), shortcut));
                return;
            }
            assigned.insert(shortcut, entry.first->text());
        }
        dialog.accept();
    });
    layout->addWidget(buttons);

    if (dialog.exec() != QDialog::Accepted) return;
    QSettings settings;
    for (const auto &entry : editors) {
        const QString id = entry.first->property("shortcutId").toString();
        const QKeySequence sequence = entry.second->keySequence();
        entry.first->setShortcut(sequence);
        entry.first->setToolTip(sequence.isEmpty() ? entry.first->text()
            : QStringLiteral("%1 (%2)").arg(entry.first->text(), sequence.toString(QKeySequence::NativeText)));
        settings.setValue(QStringLiteral("shortcuts/") + id,
            sequence.toString(QKeySequence::PortableText));
    }
}
