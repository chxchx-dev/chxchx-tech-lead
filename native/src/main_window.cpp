#include "main_window.hpp"
#include "main_window_areas.hpp"
#include "integrations/bridge_client.hpp"
#include "integrations/bridge_schemas.hpp"
#include "integrations/terminal_launcher.hpp"
#include "integrations/vt_terminal_widget.hpp"
#include "project_file_index.hpp"

#include <ScintillaEditBase.h>
#include <ILexer.h>
#include <Lexilla.h>
#include <Scintilla.h>
#include <ScintillaMessages.h>

#include <algorithm>
#include <array>
#include <iterator>
#include <utility>

#include <QAction>
#include <QApplication>
#include <QComboBox>
#include <QClipboard>
#include <QDateTime>
#include <QSortFilterProxyModel>
#include <QFontDatabase>
#include <QFontInfo>
#include <QDialog>
#include <QDockWidget>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QFileSystemModel>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QListWidgetItem>
#include <QMessageBox>
#include <QMenu>
#include <QPlainTextEdit>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QInputDialog>
#include <QIcon>
#include <QPushButton>
#include <QSaveFile>
#include <QSettings>
#include <QSplitter>
#include <QStackedWidget>
#include <QStatusBar>
#include <QTabBar>
#include <QTabWidget>
#include <QTextDocument>
#include <QTextCursor>
#include <QTextEdit>
#include <QTimer>
#include <QToolBar>
#include <QTreeView>
#include <QVBoxLayout>
#include <QWidget>

namespace {

constexpr int scintillaColor(unsigned int rgb)
{
    const unsigned int red = (rgb >> 16) & 0xFF;
    const unsigned int green = (rgb >> 8) & 0xFF;
    const unsigned int blue = rgb & 0xFF;
    return static_cast<int>(red | (green << 8) | (blue << 16));
}

void setEditorText(ScintillaEditBase *editor, const QString &text)
{
    const QByteArray utf8 = text.toUtf8();
    editor->send(SCI_SETTEXT, 0, reinterpret_cast<Scintilla::sptr_t>(utf8.constData()));
    editor->send(SCI_SETSAVEPOINT);
}

QString editorText(const ScintillaEditBase *editor)
{
    const auto length = editor->send(SCI_GETLENGTH);
    if (length <= 0) {
        return {};
    }
    QByteArray utf8(static_cast<qsizetype>(length + 1), '\0');
    editor->send(SCI_GETTEXT, static_cast<Scintilla::uptr_t>(utf8.size()),
        reinterpret_cast<Scintilla::sptr_t>(utf8.data()));
    return QString::fromUtf8(utf8.constData(), static_cast<qsizetype>(length));
}

QString editorTabTitle(const QString &path, bool modified)
{
    const QString title = QFileInfo(path).fileName();
    return modified ? QStringLiteral("● %1").arg(title) : title;
}

QString lexerNameForPath(const QString &path)
{
    const QString suffix = QFileInfo(path).suffix().toLower();
    if (suffix == QStringLiteral("c") || suffix == QStringLiteral("cc")
        || suffix == QStringLiteral("cpp") || suffix == QStringLiteral("cxx")
        || suffix == QStringLiteral("h") || suffix == QStringLiteral("hh")
        || suffix == QStringLiteral("hpp") || suffix == QStringLiteral("hxx")) {
        return QStringLiteral("cpp");
    }
    if (suffix == QStringLiteral("py")) return QStringLiteral("python");
    if (suffix == QStringLiteral("js") || suffix == QStringLiteral("jsx")) return QStringLiteral("javascript");
    if (suffix == QStringLiteral("ts") || suffix == QStringLiteral("tsx")) return QStringLiteral("typescript");
    if (suffix == QStringLiteral("json")) return QStringLiteral("json");
    if (suffix == QStringLiteral("yaml") || suffix == QStringLiteral("yml")) return QStringLiteral("yaml");
    if (suffix == QStringLiteral("toml")) return QStringLiteral("toml");
    if (suffix == QStringLiteral("md") || suffix == QStringLiteral("markdown")) return QStringLiteral("markdown");
    if (suffix == QStringLiteral("html") || suffix == QStringLiteral("htm")) return QStringLiteral("html");
    if (suffix == QStringLiteral("xml")) return QStringLiteral("xml");
    if (suffix == QStringLiteral("css")) return QStringLiteral("css");
    if (suffix == QStringLiteral("sql")) return QStringLiteral("sql");
    if (suffix == QStringLiteral("sh")) return QStringLiteral("bash");
    if (suffix == QStringLiteral("cmake") || QFileInfo(path).fileName() == QStringLiteral("CMakeLists.txt")) {
        return QStringLiteral("cmake");
    }
    return {};
}

void configureCodeEditor(ScintillaEditBase *editor, const QString &path, bool wordWrapEnabled)
{
    editor->send(SCI_SETCODEPAGE, SC_CP_UTF8);
    editor->send(SCI_SETUNDOCOLLECTION, 1);
    editor->send(SCI_SETMARGINWIDTHN, 0, 54);
    editor->send(SCI_SETMARGINTYPEN, 0, SC_MARGIN_NUMBER);
    editor->send(SCI_SETMARGINWIDTHN, 1, 14);
    editor->send(SCI_SETMARGINTYPEN, 1, SC_MARGIN_SYMBOL);
    editor->send(SCI_SETMARGINMASKN, 1, SC_MASK_FOLDERS);
    editor->send(SCI_SETMARGINSENSITIVEN, 1, 1);
    editor->send(SCI_SETPROPERTY, reinterpret_cast<Scintilla::uptr_t>("fold"),
        reinterpret_cast<Scintilla::sptr_t>("1"));
    editor->send(SCI_SETPROPERTY, reinterpret_cast<Scintilla::uptr_t>("fold.compact"),
        reinterpret_cast<Scintilla::sptr_t>("1"));

    const QFont font = QFontDatabase::systemFont(QFontDatabase::FixedFont);
    const QByteArray family = QFontInfo(font).family().toUtf8();
    editor->send(SCI_STYLESETFONT, STYLE_DEFAULT, reinterpret_cast<Scintilla::sptr_t>(family.constData()));
    editor->send(SCI_STYLESETSIZE, STYLE_DEFAULT, font.pointSize());
    editor->send(SCI_STYLESETFORE, STYLE_DEFAULT, scintillaColor(0xB9C6D8));
    editor->send(SCI_STYLESETBACK, STYLE_DEFAULT, scintillaColor(0x0E1728));
    editor->send(SCI_STYLECLEARALL);
    editor->send(SCI_STYLESETFORE, STYLE_LINENUMBER, scintillaColor(0x7189A6));
    editor->send(SCI_STYLESETBACK, STYLE_LINENUMBER, scintillaColor(0x122941));
    editor->send(SCI_SETCARETFORE, scintillaColor(0x63E6EE));
    editor->send(SCI_SETSELFORE, 1, scintillaColor(0xFFFFFF));
    editor->send(SCI_SETSELBACK, 1, scintillaColor(0x17485B));
    editor->send(SCI_SETCARETLINEVISIBLE, 1);
    editor->send(SCI_SETCARETLINEBACK, scintillaColor(0x112338));
    editor->send(SCI_SETINDENTATIONGUIDES, SC_IV_LOOKBOTH);
    editor->send(SCI_SETTABWIDTH, 4);
    editor->send(SCI_SETWRAPMODE, wordWrapEnabled ? SC_WRAP_WORD : SC_WRAP_NONE);
    editor->send(SCI_SETHSCROLLBAR, wordWrapEnabled ? 0 : 1);

    const QByteArray lexerName = lexerNameForPath(path).toLatin1();
    if (!lexerName.isEmpty()) {
        Scintilla::ILexer5 *lexer = CreateLexer(lexerName.constData());
        if (lexer != nullptr) {
            editor->send(SCI_SETILEXER, 0, reinterpret_cast<Scintilla::sptr_t>(lexer));
            editor->send(SCI_STYLECLEARALL);
            const std::array<std::pair<int, int>, 11> syntaxColors = {{
                {1, 0x8B949E}, {2, 0x8B949E}, {3, 0x8B949E}, {4, 0xD29922},
                {5, 0x79C0FF}, {6, 0x98B5CC}, {7, 0x98B5CC}, {8, 0xFF7B72},
                {9, 0xD2A8FF}, {10, 0xD2A8FF}, {11, 0x7EE787},
            }};
            for (const auto &[style, color] : syntaxColors) {
                editor->send(SCI_STYLESETFORE, static_cast<Scintilla::uptr_t>(style), scintillaColor(color));
            }
        }
    }
    editor->send(SCI_STYLESETFORE, STYLE_LINENUMBER, scintillaColor(0x7896B5));
    editor->send(SCI_STYLESETBACK, STYLE_LINENUMBER, scintillaColor(0x122941));
    editor->send(SCI_SETMARGINBACKN, 0, scintillaColor(0x122941));
}

} // namespace

MainWindow::MainWindow(QString projectPath, QWidget *parent)
    : QMainWindow(parent), m_projectPath(QFileInfo(projectPath).absoluteFilePath())
{
    setWindowTitle(QStringLiteral("ChxChx Studio — %1").arg(QFileInfo(m_projectPath).fileName()));
    setWindowIcon(QIcon(QStringLiteral(":/brand/logo-min.png")));
    resize(1440, 920);
    buildActions();
    buildLayout();
    m_fileIndex = new ProjectFileIndex(m_projectPath, this);
    statusBar()->showMessage(m_projectPath);
}

void MainWindow::setWordWrapEnabled(bool enabled)
{
    m_wordWrapEnabled = enabled;
    QSettings settings;
    settings.setValue(QStringLiteral("editor/wordWrap"), enabled);
    for (QTabWidget *tabs : editorTabGroups()) {
        for (int index = 0; index < tabs->count(); ++index) {
            auto *editor = qobject_cast<ScintillaEditBase *>(tabs->widget(index));
            if (editor == nullptr) continue;
            editor->send(SCI_SETWRAPMODE, enabled ? SC_WRAP_WORD : SC_WRAP_NONE);
            editor->send(SCI_SETHSCROLLBAR, enabled ? 0 : 1);
        }
    }
}

void MainWindow::updateEditActionState()
{
    QWidget *target = m_editTargetWidget.data();
    bool terminalFocused = false;
    bool textControlFocused = false;
    for (QWidget *widget = target; widget != nullptr; widget = widget->parentWidget()) {
        if (qobject_cast<VtTerminalWidget *>(widget) != nullptr) terminalFocused = true;
        if (qobject_cast<ScintillaEditBase *>(widget) != nullptr
            || qobject_cast<QLineEdit *>(widget) != nullptr
            || qobject_cast<QPlainTextEdit *>(widget) != nullptr
            || qobject_cast<QTextEdit *>(widget) != nullptr) {
            textControlFocused = true;
        }
    }
    const QStringList actions = {QStringLiteral("edit.undo"), QStringLiteral("edit.redo"),
        QStringLiteral("edit.cut"), QStringLiteral("edit.copy"), QStringLiteral("edit.paste"),
        QStringLiteral("edit.selectAll")};
    for (const QString &id : actions) {
        QAction *action = m_shortcutActions.value(id);
        if (action != nullptr) action->setEnabled(textControlFocused && !terminalFocused);
    }
    if (terminalFocused) {
        QAction *paste = m_shortcutActions.value(QStringLiteral("edit.paste"));
        if (paste != nullptr) paste->setEnabled(true);
    }
}

void MainWindow::performEditAction(const QString &actionId)
{
    QWidget *target = m_editTargetWidget.data();
    if (target == nullptr) return;

    VtTerminalWidget *terminal = nullptr;
    ScintillaEditBase *editor = nullptr;
    QLineEdit *lineEdit = nullptr;
    QPlainTextEdit *plainText = nullptr;
    QTextEdit *textEdit = nullptr;
    for (QWidget *widget = target; widget != nullptr; widget = widget->parentWidget()) {
        if (terminal == nullptr) terminal = qobject_cast<VtTerminalWidget *>(widget);
        if (editor == nullptr) editor = qobject_cast<ScintillaEditBase *>(widget);
        if (lineEdit == nullptr) lineEdit = qobject_cast<QLineEdit *>(widget);
        if (plainText == nullptr) plainText = qobject_cast<QPlainTextEdit *>(widget);
        if (textEdit == nullptr) textEdit = qobject_cast<QTextEdit *>(widget);
    }

    if (terminal != nullptr) {
        if (actionId == QStringLiteral("edit.paste")) {
            terminal->pasteText(QApplication::clipboard()->text());
        }
        return;
    }
    if (editor != nullptr) {
        if (actionId == QStringLiteral("edit.undo")) editor->send(SCI_UNDO);
        else if (actionId == QStringLiteral("edit.redo")) editor->send(SCI_REDO);
        else if (actionId == QStringLiteral("edit.cut")) editor->send(SCI_CUT);
        else if (actionId == QStringLiteral("edit.copy")) editor->send(SCI_COPY);
        else if (actionId == QStringLiteral("edit.paste")) editor->send(SCI_PASTE);
        else if (actionId == QStringLiteral("edit.selectAll")) editor->send(SCI_SELECTALL);
        return;
    }
    if (lineEdit != nullptr) {
        if (actionId == QStringLiteral("edit.undo")) lineEdit->undo();
        else if (actionId == QStringLiteral("edit.redo")) lineEdit->redo();
        else if (actionId == QStringLiteral("edit.cut")) lineEdit->cut();
        else if (actionId == QStringLiteral("edit.copy")) lineEdit->copy();
        else if (actionId == QStringLiteral("edit.paste")) lineEdit->paste();
        else if (actionId == QStringLiteral("edit.selectAll")) lineEdit->selectAll();
        return;
    }
    if (plainText != nullptr) {
        if (actionId == QStringLiteral("edit.undo")) plainText->undo();
        else if (actionId == QStringLiteral("edit.redo")) plainText->redo();
        else if (actionId == QStringLiteral("edit.cut")) plainText->cut();
        else if (actionId == QStringLiteral("edit.copy")) plainText->copy();
        else if (actionId == QStringLiteral("edit.paste")) plainText->paste();
        else if (actionId == QStringLiteral("edit.selectAll")) plainText->selectAll();
        return;
    }
    if (textEdit != nullptr) {
        if (actionId == QStringLiteral("edit.undo")) textEdit->undo();
        else if (actionId == QStringLiteral("edit.redo")) textEdit->redo();
        else if (actionId == QStringLiteral("edit.cut")) textEdit->cut();
        else if (actionId == QStringLiteral("edit.copy")) textEdit->copy();
        else if (actionId == QStringLiteral("edit.paste")) textEdit->paste();
        else if (actionId == QStringLiteral("edit.selectAll")) textEdit->selectAll();
    }
}

QList<QTabWidget *> MainWindow::editorTabGroups() const
{
    QList<QTabWidget *> groups{m_editorTabs};
    if (m_secondaryEditorTabs != nullptr) groups.append(m_secondaryEditorTabs);
    return groups;
}

QTabWidget *MainWindow::activeEditorTabs() const
{
    return m_activeEditorTabs != nullptr ? m_activeEditorTabs : m_editorTabs;
}

QTabWidget *MainWindow::tabGroupFor(QWidget *page) const
{
    for (QTabWidget *tabs : editorTabGroups()) {
        if (tabs->indexOf(page) >= 0) return tabs;
    }
    return nullptr;
}

void MainWindow::showTabContextMenu(QTabWidget *tabs, const QPoint &position)
{
    const int index = tabs->tabBar()->tabAt(position);
    if (index < 0 || tabs->widget(index) == m_mainPages) return;

    QMenu menu(this);
    QAction *splitRight = menu.addAction(QStringLiteral("Dividir a la derecha"));
    QAction *splitBelow = menu.addAction(QStringLiteral("Dividir abajo"));
    QAction *moveOther = nullptr;
    QAction *closeGroup = nullptr;
    if (m_secondaryEditorTabs->isVisible()) {
        menu.addSeparator();
        moveOther = menu.addAction(QStringLiteral("Mover pestaña al otro grupo"));
        closeGroup = menu.addAction(QStringLiteral("Cerrar grupo dividido"));
    }
    QAction *selected = menu.exec(tabs->tabBar()->mapToGlobal(position));
    if (selected == splitRight) {
        moveTabToOtherGroup(tabs, index, Qt::Horizontal);
    } else if (selected == splitBelow) {
        moveTabToOtherGroup(tabs, index, Qt::Vertical);
    } else if (selected == moveOther) {
        moveTabToOtherGroup(tabs, index, m_editorSplit->orientation());
    } else if (selected == closeGroup) {
        closeSecondaryTabGroup();
    }
}

void MainWindow::moveTabToOtherGroup(QTabWidget *source, int index, Qt::Orientation orientation)
{
    if (index < 0 || index >= source->count() || source->widget(index) == m_mainPages) return;
    QTabWidget *destination = source == m_editorTabs ? m_secondaryEditorTabs : m_editorTabs;
    const bool openingSplit = destination == m_secondaryEditorTabs && !destination->isVisible();
    m_editorSplit->setOrientation(orientation);
    if (destination == m_secondaryEditorTabs) {
        destination->show();
        if (openingSplit) {
            const int extent = orientation == Qt::Horizontal ? m_editorSplit->width() : m_editorSplit->height();
            m_editorSplit->setSizes({extent / 2, extent / 2});
        }
    }

    QWidget *page = source->widget(index);
    const QString title = source->tabText(index);
    const QIcon icon = source->tabIcon(index);
    const QString tooltip = source->tabToolTip(index);
    source->removeTab(index);
    const int newIndex = destination->addTab(page, icon, title);
    destination->setTabToolTip(newIndex, tooltip);
    destination->setCurrentIndex(newIndex);
    m_activeEditorTabs = destination;

    if (source == m_secondaryEditorTabs && source->count() == 0) {
        source->hide();
        m_activeEditorTabs = m_editorTabs;
    }
}

void MainWindow::closeSecondaryTabGroup()
{
    while (m_secondaryEditorTabs->count() > 0) {
        QWidget *page = m_secondaryEditorTabs->widget(0);
        const QString title = m_secondaryEditorTabs->tabText(0);
        const QIcon icon = m_secondaryEditorTabs->tabIcon(0);
        const QString tooltip = m_secondaryEditorTabs->tabToolTip(0);
        m_secondaryEditorTabs->removeTab(0);
        const int index = m_editorTabs->addTab(page, icon, title);
        m_editorTabs->setTabToolTip(index, tooltip);
    }
    m_secondaryEditorTabs->hide();
    m_activeEditorTabs = m_editorTabs;
    m_editorTabs->setCurrentIndex(m_editorTabs->count() - 1);
}

void MainWindow::openFile()
{
    const QString path = QFileDialog::getOpenFileName(this, QStringLiteral("Abrir archivo"), m_projectPath);
    if (!path.isEmpty()) {
        openPath(path);
    }
}

void MainWindow::openTreeFile(const QModelIndex &index)
{
    if (!index.isValid()) return;
    const QModelIndex sourceIndex = m_projectProxy->mapToSource(index);
    const QString path = m_fileModel->filePath(sourceIndex);
    if (m_fileModel->isDir(sourceIndex)) {
        m_projectTree->setExpanded(index, !m_projectTree->isExpanded(index));
        return;
    }
    openPath(path);
}

void MainWindow::openPath(const QString &path)
{
    for (QTabWidget *tabs : editorTabGroups()) {
        for (int index = 0; index < tabs->count(); ++index) {
            auto *editor = qobject_cast<ScintillaEditBase *>(tabs->widget(index));
            if (editor != nullptr && editor->property("filePath").toString() == path) {
                tabs->setCurrentIndex(index);
                m_activeEditorTabs = tabs;
                editor->setFocus(Qt::OtherFocusReason);
                return;
            }
        }
    }

    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        QMessageBox::warning(this, QStringLiteral("No se pudo abrir"), file.errorString());
        return;
    }
    QTabWidget *tabs = activeEditorTabs();
    auto *editor = new ScintillaEditBase(tabs);
    editor->setProperty("filePath", path);
    configureCodeEditor(editor, path, m_wordWrapEnabled);
    setEditorText(editor, QString::fromUtf8(file.readAll()));
    const int tab = tabs->addTab(editor, editorTabTitle(path, false));
    tabs->setTabToolTip(tab, path);
    tabs->setCurrentIndex(tab);
    connect(editor, &ScintillaEditBase::savePointChanged, this, [this, editor](bool dirty) {
        editor->setProperty("modified", dirty);
        QTabWidget *tabs = tabGroupFor(editor);
        const int tabIndex = tabs != nullptr ? tabs->indexOf(editor) : -1;
        if (tabs != nullptr && tabIndex >= 0) {
            const QString path = editor->property("filePath").toString();
            tabs->setTabText(tabIndex, editorTabTitle(path, dirty));
        }
    });
}

void MainWindow::saveFile()
{
    if (currentEditor() == nullptr) {
        return;
    }
    if (currentFilePath().isEmpty()) {
        saveFileAs();
        return;
    }
    QSaveFile file(currentFilePath());
    if (!file.open(QIODevice::WriteOnly) || file.write(editorText(currentEditor()).toUtf8()) < 0 || !file.commit()) {
        QMessageBox::warning(this, QStringLiteral("No se pudo guardar"), file.errorString());
        return;
    }
    currentEditor()->send(SCI_SETSAVEPOINT);
    QTabWidget *tabs = tabGroupFor(currentEditor());
    if (tabs != nullptr) {
        tabs->setTabText(tabs->indexOf(currentEditor()), editorTabTitle(currentFilePath(), false));
    }
    statusBar()->showMessage(QStringLiteral("Guardado: %1").arg(currentFilePath()), 3000);
}

void MainWindow::saveFileAs()
{
    if (currentEditor() == nullptr) {
        return;
    }
    const QString path = QFileDialog::getSaveFileName(this, QStringLiteral("Guardar archivo"), m_projectPath);
    if (path.isEmpty()) {
        return;
    }
    currentEditor()->setProperty("filePath", path);
    saveFile();
}

void MainWindow::findInCurrentFile()
{
    auto *editor = currentEditor();
    if (editor == nullptr) {
        statusBar()->showMessage(QStringLiteral("Abre un archivo para buscar texto."), 3000);
        return;
    }
    bool accepted = false;
    const QString query = QInputDialog::getText(this, QStringLiteral("Buscar en archivo"),
        QStringLiteral("Texto"), QLineEdit::Normal, QString(), &accepted).trimmed();
    if (!accepted || query.isEmpty()) {
        return;
    }
    const QByteArray utf8 = query.toUtf8();
    editor->send(SCI_SEARCHANCHOR);
    auto position = editor->sends(SCI_SEARCHNEXT, SCFIND_NONE, utf8.constData());
    if (position < 0) {
        editor->send(SCI_DOCUMENTSTART);
        editor->send(SCI_SEARCHANCHOR);
        position = editor->sends(SCI_SEARCHNEXT, SCFIND_NONE, utf8.constData());
    }
    if (position < 0) {
        statusBar()->showMessage(QStringLiteral("No se encontró: %1").arg(query), 4000);
    } else {
        editor->send(SCI_SETSEL, static_cast<Scintilla::uptr_t>(position), position + utf8.size());
        editor->setFocus();
    }
}

void MainWindow::closeEditorTab(QTabWidget *tabs, int index)
{
    if (tabs == nullptr || index < 0 || index >= tabs->count()) {
        return;
    }
    if (tabs->widget(index) == m_mainPages) {
        return;
    }
    auto *editor = qobject_cast<ScintillaEditBase *>(tabs->widget(index));
    if (editor != nullptr && editor->property("modified").toBool()) {
        const auto result = QMessageBox::question(this, QStringLiteral("Cambios sin guardar"),
            QStringLiteral("¿Cerrar esta pestaña y descartar los cambios?"),
            QMessageBox::Discard | QMessageBox::Cancel, QMessageBox::Cancel);
        if (result != QMessageBox::Discard) {
            return;
        }
    }
    QWidget *page = tabs->widget(index);
    tabs->removeTab(index);
    page->deleteLater();
    if (tabs == m_secondaryEditorTabs && tabs->count() == 0) {
        tabs->hide();
        m_activeEditorTabs = m_editorTabs;
    }
}

void MainWindow::selectArea(int row)
{
    if (row < 0 || row >= m_areaList->count()) {
        return;
    }
    const auto *item = m_areaList->item(row);
    m_currentArea = item->data(Qt::UserRole).toString();
    const bool dashboardArea = m_currentArea == QStringLiteral("overview")
        || m_currentArea == QStringLiteral("project") || m_currentArea == QStringLiteral("agents")
        || m_currentArea == QStringLiteral("processes") || m_currentArea == QStringLiteral("projects")
        || m_currentArea == QStringLiteral("skills") || m_currentArea == QStringLiteral("packs")
        || m_currentArea == QStringLiteral("resources");
    if (dashboardArea) {
        m_mainPages->setCurrentWidget(m_dashboardPage);
        m_dashboardTitle->setText(item->text());
        m_dashboardSummary->setText(QStringLiteral("Consultando el estado del proyecto…"));
        m_dashboardItems->clear();
    } else if (m_currentArea == QStringLiteral("handoff")) {
        m_mainPages->setCurrentWidget(m_handoffPage);
    } else if (m_currentArea == QStringLiteral("memory")) {
        m_mainPages->setCurrentWidget(m_memoryPage);
    } else if (m_currentArea == QStringLiteral("chats")) {
        m_mainPages->setCurrentWidget(m_chatsPage);
    } else if (m_currentArea == QStringLiteral("errors")) {
        m_mainPages->setCurrentWidget(m_errorsPage);
    } else if (m_currentArea == QStringLiteral("setup")) {
        m_mainPages->setCurrentWidget(m_setupPage);
    } else if (m_currentArea == QStringLiteral("guide")) {
        m_mainPages->setCurrentWidget(m_guidePage);
    } else if (m_currentArea == QStringLiteral("brand")) {
        m_mainPages->setCurrentWidget(m_brandPage);
    } else {
        m_mainPages->setCurrentWidget(m_dashboardPage);
    }
    const int sectionTab = m_editorTabs->indexOf(m_mainPages);
    if (sectionTab >= 0) {
        m_editorTabs->setCurrentIndex(sectionTab);
    }
    const auto area = std::find_if(std::begin(areas), std::end(areas), [this](const Area &candidate) {
        return m_currentArea == QString::fromUtf8(candidate.id);
    });
    if (area != std::end(areas)) {
        m_areaDescription->setText(QString::fromUtf8(area->description));
    }
    configureAreaActions();
    refreshArea();
}

void MainWindow::openCommandPalette()
{
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("Paleta de comandos"));
    dialog.setMinimumSize(560, 460);
    auto *layout = new QVBoxLayout(&dialog);
    auto *query = new QLineEdit(&dialog);
    query->setPlaceholderText(QStringLiteral("Buscar vista o acción…"));
    auto *commands = new QListWidget(&dialog);
    const QList<QPair<QString, QString>> entries = {
        {QStringLiteral("Ir a Inicio"), QStringLiteral("area:overview")},
        {QStringLiteral("Ir a Proyecto"), QStringLiteral("area:project")},
        {QStringLiteral("Ir a Agentes"), QStringLiteral("area:agents")},
        {QStringLiteral("Ir a Procesos"), QStringLiteral("area:processes")},
        {QStringLiteral("Ir a Proyectos"), QStringLiteral("area:projects")},
        {QStringLiteral("Ir a Skills"), QStringLiteral("area:skills")},
        {QStringLiteral("Ir a Tech Packs"), QStringLiteral("area:packs")},
        {QStringLiteral("Ir a Recursos"), QStringLiteral("area:resources")},
        {QStringLiteral("Ir a Handoff"), QStringLiteral("area:handoff")},
        {QStringLiteral("Ir a Memoria"), QStringLiteral("area:memory")},
        {QStringLiteral("Ir a Chats"), QStringLiteral("area:chats")},
        {QStringLiteral("Ir a Errores"), QStringLiteral("area:errors")},
        {QStringLiteral("Ir a Configuración"), QStringLiteral("area:setup")},
        {QStringLiteral("Abrir Guía"), QStringLiteral("area:guide")},
        {QStringLiteral("Ver Marca CHXCHX"), QStringLiteral("area:brand")},
        {QStringLiteral("Abrir archivo…"), QStringLiteral("open")},
        {QStringLiteral("Buscar archivo del proyecto…"), QStringLiteral("file-search")},
        {QStringLiteral("Guardar archivo"), QStringLiteral("save")},
        {QStringLiteral("Guardar archivo como…"), QStringLiteral("save-as")},
        {QStringLiteral("Buscar en archivo…"), QStringLiteral("find")},
        {QStringLiteral("Actualizar vista actual"), QStringLiteral("refresh")},
        {QStringLiteral("Ejecutar acción principal de la vista"), QStringLiteral("primary")},
        {QStringLiteral("Ejecutar acción secundaria de la vista"), QStringLiteral("secondary")},
        {QStringLiteral("Ejecutar tercera acción de la vista"), QStringLiteral("tertiary")},
        {QStringLiteral("Ejecutar cuarta acción de la vista"), QStringLiteral("quaternary")},
        {QStringLiteral("Abrir terminal integrada del workspace"), QStringLiteral("workspace-terminal")},
        {QStringLiteral("Adjuntar a la terminal del workspace"), QStringLiteral("workspace-attach")},
        {QStringLiteral("Adjuntar a la terminal del agente seleccionado"), QStringLiteral("agent-attach")},
    };
    for (const auto &entry : entries) {
        auto *item = new QListWidgetItem(entry.first, commands);
        item->setData(Qt::UserRole, entry.second);
    }
    layout->addWidget(query);
    layout->addWidget(commands, 1);
    connect(query, &QLineEdit::textChanged, &dialog, [query, commands] {
        const QString filter = query->text().trimmed();
        for (int row = 0; row < commands->count(); ++row) {
            auto *item = commands->item(row);
            item->setHidden(!item->text().contains(filter, Qt::CaseInsensitive));
        }
        for (int row = 0; row < commands->count(); ++row) {
            if (!commands->item(row)->isHidden()) {
                commands->setCurrentRow(row);
                break;
            }
        }
    });
    connect(commands, &QListWidget::itemActivated, &dialog, [&dialog](QListWidgetItem *) {
        dialog.accept();
    });
    connect(query, &QLineEdit::returnPressed, &dialog, &QDialog::accept);
    commands->setCurrentRow(0);
    query->setFocus();
    if (dialog.exec() != QDialog::Accepted || commands->currentItem() == nullptr) {
        return;
    }
    executePaletteCommand(commands->currentItem());
}

void MainWindow::executePaletteCommand(QListWidgetItem *item)
{
    const QString command = item->data(Qt::UserRole).toString();
    if (command.startsWith(QStringLiteral("area:"))) {
        const QString area = command.mid(5);
        for (int row = 0; row < m_areaList->count(); ++row) {
            if (m_areaList->item(row)->data(Qt::UserRole).toString() == area) {
                m_areaList->setCurrentRow(row);
                return;
            }
        }
    } else if (command == QStringLiteral("open")) {
        openFile();
    } else if (command == QStringLiteral("file-search")) {
        searchProjectFiles();
    } else if (command == QStringLiteral("save")) {
        saveFile();
    } else if (command == QStringLiteral("save-as")) {
        saveFileAs();
    } else if (command == QStringLiteral("find")) {
        findInCurrentFile();
    } else if (command == QStringLiteral("refresh")) {
        refreshArea();
    } else if (command == QStringLiteral("primary")) {
        if (m_primaryAction->isVisible() && m_primaryAction->isEnabled()) {
            performPrimaryAreaAction();
        }
    } else if (command == QStringLiteral("secondary")) {
        if (m_secondaryAction->isVisible() && m_secondaryAction->isEnabled()) {
            performSecondaryAreaAction();
        }
    } else if (command == QStringLiteral("tertiary")) {
        if (m_tertiaryAction->isVisible() && m_tertiaryAction->isEnabled()) {
            performTertiaryAreaAction();
        }
    } else if (command == QStringLiteral("quaternary")) {
        if (m_quaternaryAction->isVisible() && m_quaternaryAction->isEnabled()) {
            performQuaternaryAreaAction();
        }
    } else if (command == QStringLiteral("workspace-terminal")) {
        openNewWorkspaceTerminal();
    } else if (command == QStringLiteral("workspace-attach")) {
        attachWorkspaceTerminal();
    } else if (command == QStringLiteral("agent-attach")) {
        attachAgentTerminal();
    }
}

void MainWindow::refreshArea()
{
    if (m_bridgeClient->isRunning()) {
        m_refreshQueued = true;
        statusBar()->showMessage(QStringLiteral("Espera a que termine el comando actual."));
        return;
    }
    const QStringList arguments = readCommandForArea(m_currentArea);
    if (arguments.isEmpty()) {
        appendOutput(QStringLiteral("Esta vista se integrará con el bridge compartido de ChxChx."));
        return;
    }
    m_commandArea = m_currentArea;
    runCommand(arguments);
}

void MainWindow::runCommand(const QStringList &arguments)
{
    if (m_bridgeClient->isRunning()) {
        statusBar()->showMessage(QStringLiteral("Espera a que termine el comando actual."));
        return;
    }
    m_commandArea = m_currentArea;
    if (!m_bridgeClient->execute(arguments)) {
        return;
    }
    statusBar()->showMessage(QStringLiteral("Consultando ChxChx…"));
}

void MainWindow::commandFailedToStart(const QString &reason)
{
    m_previewPending = false;
    m_governorRetryPending = false;
    m_confirmedActionArguments.clear();
    m_forceActionArguments.clear();
    m_confirmedActionTitle.clear();
    m_pendingProjectPath.clear();
    appendOutput(QStringLiteral("No se pudo iniciar chxchx-tech: %1").arg(reason));
    statusBar()->showMessage(QStringLiteral("No se pudo iniciar ChxChx CLI."), 6000);
}

QStringList MainWindow::readCommandForArea(const QString &area) const
{
    if (area == QStringLiteral("overview") || area == QStringLiteral("project")
        || area == QStringLiteral("agents") || area == QStringLiteral("processes")
        || area == QStringLiteral("projects")) {
        return {QStringLiteral("bridge"), QStringLiteral("status"), m_projectPath};
    }
    if (area == QStringLiteral("resources")) {
        return {QStringLiteral("bridge"), QStringLiteral("resources"), m_projectPath};
    }
    if (area == QStringLiteral("skills") || area == QStringLiteral("packs")) {
        return {QStringLiteral("bridge"), QStringLiteral("status"), m_projectPath};
    }
    if (area == QStringLiteral("handoff")) {
        return {QStringLiteral("bridge"), QStringLiteral("handoff"), m_projectPath};
    }
    if (area == QStringLiteral("memory")) {
        return {QStringLiteral("bridge"), QStringLiteral("memory"), m_projectPath,
            QStringLiteral("--query"), m_memorySearch->text()};
    }
    if (area == QStringLiteral("chats")) {
        return {QStringLiteral("bridge"), QStringLiteral("chats"), m_projectPath,
            QStringLiteral("--query"), m_chatSearch->text()};
    }
    if (area == QStringLiteral("errors")) {
        return {QStringLiteral("bridge"), QStringLiteral("errors"), m_projectPath};
    }
    return {};
}

void MainWindow::finishCommand(int exitCode, QProcess::ExitStatus status)
{
    QString output = QString::fromUtf8(m_bridgeClient->readStandardOutput()).trimmed();
    const QString error = QString::fromUtf8(m_bridgeClient->readStandardError()).trimmed();
    const auto json = QJsonDocument::fromJson(output.toUtf8());
    if (json.isObject()) {
        const QJsonObject payload = json.object();
        const auto schema = payload.value(QStringLiteral("schema")).toString();
        const auto version = payload.value(QStringLiteral("schema_version"));
        if (!version.isDouble() || version.toInt(-1) != BridgeSchemas::Version) {
            output = QStringLiteral("Contrato bridge incompatible: versión ausente o no compatible (%1).")
                .arg(version.isDouble() ? QString::number(version.toInt()) : QStringLiteral("inválida"));
        } else if (schema == BridgeSchemas::ResourcesOverview) {
            showAreaDashboard(payload, m_commandArea);
            output = formatResourcesOverview(payload);
        } else if (schema == BridgeSchemas::ProjectStatus) {
            const QJsonObject workspace = payload.value(QStringLiteral("workspace")).toObject();
            m_projectTrusted = workspace.value(QStringLiteral("trusted")).toBool();
            m_workspaceStatus = workspace.value(QStringLiteral("status")).toString();
            updateAreaTargets(payload, m_commandArea);
            updateAreaActionState();
            showAreaDashboard(payload, m_commandArea);
            output = formatBridgeStatus(payload, m_commandArea);
        } else if (schema == BridgeSchemas::Handoff) {
            showHandoff(payload);
            output = payload.value(QStringLiteral("exists")).toBool()
                ? QStringLiteral("Handoff cargado en modo lectura.")
                : QStringLiteral("Este proyecto todavía no tiene handoff.");
        } else if (schema == BridgeSchemas::Memory) {
            showMemory(payload);
            output = QStringLiteral("Memoria local actualizada: %1 nota(s).")
                .arg(payload.value(QStringLiteral("notes")).toArray().size());
        } else if (schema == BridgeSchemas::Conversations) {
            showConversations(payload);
            output = QStringLiteral("Conversaciones locales: %1.")
                .arg(payload.value(QStringLiteral("conversations")).toArray().size());
        } else if (schema == BridgeSchemas::Conversation) {
            showConversation(payload);
            output = QStringLiteral("Conversación cargada en solo lectura.");
        } else if (schema == BridgeSchemas::Errors) {
            showErrors(payload);
            output = QStringLiteral("Errores locales: %1.")
                .arg(payload.value(QStringLiteral("errors")).toArray().size());
        } else if (schema == BridgeSchemas::Error) {
            output = payload.value(QStringLiteral("error")).toString();
        } else {
            output = QStringLiteral("Contrato bridge desconocido: %1.").arg(schema.isEmpty()
                ? QStringLiteral("falta el schema") : schema);
        }
    }
    appendOutput(output.isEmpty() ? error : output + (error.isEmpty() ? QString() : QStringLiteral("\n") + error));
    if (m_previewPending) {
        m_previewPending = false;
        if (status != QProcess::NormalExit || exitCode != 0) {
            m_confirmedActionArguments.clear();
            m_forceActionArguments.clear();
            m_confirmedActionTitle.clear();
            m_pendingProjectPath.clear();
            statusBar()->showMessage(QStringLiteral("No se pudo previsualizar la acción (%1)").arg(exitCode), 5000);
            schedulePendingRefresh();
            return;
        }
        const auto result = QMessageBox::question(
            this,
            m_confirmedActionTitle,
            QStringLiteral("Previsualización:\n\n%1\n\n¿Confirmas esta operación?").arg(output),
            QMessageBox::Yes | QMessageBox::Cancel,
            QMessageBox::Cancel);
        if (result != QMessageBox::Yes) {
            m_confirmedActionArguments.clear();
            m_forceActionArguments.clear();
            m_confirmedActionTitle.clear();
            m_pendingProjectPath.clear();
            statusBar()->showMessage(QStringLiteral("Acción cancelada; no se realizaron cambios."), 4000);
            schedulePendingRefresh();
            return;
        }
        m_refreshAfterAction = true;
        const auto arguments = m_confirmedActionArguments;
        m_confirmedActionArguments.clear();
        m_confirmedActionTitle.clear();
        if (!arguments.isEmpty()
            && (arguments.first() == QStringLiteral("__launch_embedded_agent__")
                || arguments.first() == QStringLiteral("__launch_embedded_agents__"))) {
            m_forceActionArguments.clear();
            if (output.contains(QStringLiteral("RAM Governor"))) {
                const auto governorResult = QMessageBox::warning(
                    this, QStringLiteral("RAM Governor"),
                    QStringLiteral("El preflight detectó que el inicio supera el presupuesto configurado:\n\n%1\n\n¿Confirmas iniciar la sesión de todas formas?")
                        .arg(output),
                    QMessageBox::Yes | QMessageBox::Cancel, QMessageBox::Cancel);
                if (governorResult != QMessageBox::Yes) {
                    m_refreshAfterAction = false;
                    statusBar()->showMessage(QStringLiteral("Inicio cancelado por el RAM Governor."), 5000);
                    return;
                }
            }

            if (arguments.first() == QStringLiteral("__launch_embedded_agent__") && arguments.size() >= 2) {
                const auto agent = QJsonDocument::fromJson(arguments.at(1).toUtf8()).object();
                QStringList agentArguments;
                for (const auto &value : agent.value(QStringLiteral("arguments")).toArray())
                    agentArguments << value.toString();
                createEmbeddedAgentSession(agent.value(QStringLiteral("id")).toString(),
                    agent.value(QStringLiteral("program")).toString(), agentArguments,
                    agent.value(QStringLiteral("cwd")).toString(),
                    agent.value(QStringLiteral("new_chat")).toBool());
            } else if (arguments.first() == QStringLiteral("__launch_embedded_agents__")
                && arguments.size() >= 2) {
                const auto agents = QJsonDocument::fromJson(arguments.at(1).toUtf8()).array();
                for (const auto &value : agents) {
                    const auto agent = value.toObject();
                    QStringList agentArguments;
                    const QJsonArray command = agent.value(QStringLiteral("command")).toArray();
                    for (qsizetype index = 1; index < command.size(); ++index)
                        agentArguments << command.at(index).toString();
                    if (command.isEmpty()) continue;
                    createEmbeddedAgentSession(agent.value(QStringLiteral("id")).toString(),
                        command.first().toString(), agentArguments,
                        agent.value(QStringLiteral("cwd")).toString(), false);
                }
            }
            schedulePendingRefresh();
            return;
        }
        if (!arguments.isEmpty()
            && arguments.first() == QStringLiteral("__launch_embedded_workspace_terminal__")) {
            m_forceActionArguments.clear();
            createEmbeddedWorkspaceTerminal();
            statusBar()->showMessage(QStringLiteral("Terminal integrada lista."), 4000);
            schedulePendingRefresh();
            return;
        }
        if (!arguments.isEmpty() && arguments.first() == QStringLiteral("__launch_terminal__")) {
            m_forceActionArguments.clear();
            m_refreshAfterAction = true;
            QStringList terminalCommand = arguments.mid(1);
            if (output.contains(QStringLiteral("RAM Governor"))
                && terminalCommand.contains(QStringLiteral("attach"))
                && terminalCommand.contains(QStringLiteral("agent"))) {
                const auto governorResult = QMessageBox::question(
                    this, QStringLiteral("RAM Governor"),
                    QStringLiteral("La preparación del agente supera el presupuesto configurado:\n\n%1\n\n¿Continuar con --force?").arg(output),
                    QMessageBox::Yes | QMessageBox::Cancel, QMessageBox::Cancel);
                if (governorResult != QMessageBox::Yes) {
                    statusBar()->showMessage(QStringLiteral("Attach cancelado por el RAM Governor."), 5000);
                    m_refreshAfterAction = false;
                    return;
                }
                terminalCommand << QStringLiteral("--force");
            }
            QString launchError;
            const bool launched = launchExternalTerminal(terminalCommand, m_projectPath, &launchError);
            statusBar()->showMessage(launched
                ? QStringLiteral("Terminal Zellij abierta en un emulador externo.")
                : launchError, 6000);
            schedulePendingRefresh();
            return;
        }
        m_governorRetryPending = !m_forceActionArguments.isEmpty();
        runCommand(arguments);
        statusBar()->showMessage(QStringLiteral("Ejecutando operación confirmada…"));
        return;
    }
    if (m_governorRetryPending) {
        const QString combinedOutput = output + QLatin1Char('\n') + error;
        if (status == QProcess::NormalExit && exitCode == 2
            && combinedOutput.contains(QStringLiteral("RAM Governor"))
            && combinedOutput.contains(QStringLiteral("--force"))) {
            const auto result = QMessageBox::question(
                this,
                QStringLiteral("RAM Governor"),
                QStringLiteral("El estado actual supera el presupuesto:\n\n%1\n\n¿Confirmas continuar pese al aviso del RAM Governor?")
                    .arg(combinedOutput),
                QMessageBox::Yes | QMessageBox::Cancel,
                QMessageBox::Cancel);
            m_governorRetryPending = false;
            if (result == QMessageBox::Yes) {
                const auto forceArguments = m_forceActionArguments;
                m_forceActionArguments.clear();
                runCommand(forceArguments);
                return;
            }
            m_forceActionArguments.clear();
            m_pendingProjectPath.clear();
            statusBar()->showMessage(QStringLiteral("Inicio cancelado por el RAM Governor."), 5000);
            schedulePendingRefresh();
            return;
        }
        m_governorRetryPending = false;
        m_forceActionArguments.clear();
    }
    if (!m_pendingProjectPath.isEmpty()) {
        if (status == QProcess::NormalExit && exitCode == 0) {
            setProjectRoot(m_pendingProjectPath);
        }
        m_pendingProjectPath.clear();
    }
    statusBar()->showMessage(status == QProcess::NormalExit && exitCode == 0
        ? QStringLiteral("Actualizado")
        : QStringLiteral("ChxChx terminó con error (%1)").arg(exitCode), 5000);
    schedulePendingRefresh();
}

void MainWindow::setProjectRoot(const QString &path)
{
    m_projectPath = QFileInfo(path).absoluteFilePath();
    m_bridgeClient->setWorkingDirectory(m_projectPath);
    m_fileModel->setRootPath(m_projectPath);
    m_projectTree->setRootIndex(m_fileModel->index(m_projectPath));
    setWindowTitle(QStringLiteral("ChxChx Studio — %1").arg(QFileInfo(m_projectPath).fileName()));
    statusBar()->showMessage(QStringLiteral("Proyecto activo: %1").arg(m_projectPath), 5000);
}

bool MainWindow::runPreview(
    const QStringList &previewArguments,
    const QStringList &actionArguments,
    const QStringList &forceArguments,
    const QString &title)
{
    if (m_bridgeClient->isRunning()) {
        statusBar()->showMessage(QStringLiteral("Espera a que termine el comando actual."));
        return false;
    }
    m_confirmedActionArguments = actionArguments;
    m_forceActionArguments = forceArguments;
    m_confirmedActionTitle = title;
    m_previewPending = true;
    runCommand(previewArguments);
    return true;
}

void MainWindow::schedulePendingRefresh()
{
    if (!m_refreshAfterAction && !m_refreshQueued) {
        return;
    }
    m_refreshAfterAction = false;
    m_refreshQueued = false;
    QTimer::singleShot(0, this, &MainWindow::refreshArea);
}

void MainWindow::appendOutput(const QString &text)
{
    m_output->appendPlainText(QStringLiteral("[%1] %2")
        .arg(QDateTime::currentDateTime().toString(QStringLiteral("HH:mm:ss")), text));
}

ScintillaEditBase *MainWindow::currentEditor() const
{
    return qobject_cast<ScintillaEditBase *>(activeEditorTabs()->currentWidget());
}

QString MainWindow::currentFilePath() const
{
    const auto *editor = currentEditor();
    return editor == nullptr ? QString() : editor->property("filePath").toString();
}
