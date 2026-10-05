#include "main_window.hpp"
#include "main_window_areas.hpp"
#include "integrations/bridge_client.hpp"
#include "integrations/bridge_schemas.hpp"
#include "integrations/terminal_launcher.hpp"

#include <ScintillaEditBase.h>
#include <ILexer.h>
#include <Lexilla.h>
#include <Scintilla.h>
#include <ScintillaMessages.h>

#include <algorithm>
#include <array>
#include <functional>
#include <iterator>
#include <utility>

#include <QAction>
#include <QComboBox>
#include <QDateTime>
#include <QFontDatabase>
#include <QFontInfo>
#include <QDialog>
#include <QDockWidget>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QFileSystemModel>
#include <QKeySequence>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QListWidgetItem>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QInputDialog>
#include <QPushButton>
#include <QSaveFile>
#include <QSplitter>
#include <QStackedWidget>
#include <QStatusBar>
#include <QTabWidget>
#include <QTextDocument>
#include <QTextCursor>
#include <QTimer>
#include <QToolBar>
#include <QTreeView>
#include <QVBoxLayout>
#include <QWidget>

namespace {

QAction *makeAction(
    QObject *parent,
    const QString &text,
    const QKeySequence &shortcut,
    const std::function<void()> &callback)
{
    auto *action = new QAction(text, parent);
    action->setShortcut(shortcut);
    QObject::connect(action, &QAction::triggered, parent, callback);
    return action;
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

void configureCodeEditor(ScintillaEditBase *editor, const QString &path)
{
    editor->send(SCI_SETCODEPAGE, SC_CP_UTF8);
    editor->send(SCI_SETUNDOCOLLECTION, 1);
    editor->send(SCI_SETMARGINWIDTHN, 0, 44);
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
    editor->send(SCI_STYLESETFORE, STYLE_DEFAULT, 0xD8DEE9);
    editor->send(SCI_STYLESETBACK, STYLE_DEFAULT, 0x20242B);
    editor->send(SCI_STYLECLEARALL);
    editor->send(SCI_STYLESETFORE, STYLE_LINENUMBER, 0x7F8998);
    editor->send(SCI_STYLESETBACK, STYLE_LINENUMBER, 0x292E36);
    editor->send(SCI_SETCARETFORE, 0xE6EDF3);
    editor->send(SCI_SETSELFORE, 1, 0xFFFFFF);
    editor->send(SCI_SETSELBACK, 1, 0x355B83);
    editor->send(SCI_SETCARETLINEVISIBLE, 1);
    editor->send(SCI_SETCARETLINEBACK, 0x292E36);
    editor->send(SCI_SETINDENTATIONGUIDES, SC_IV_LOOKBOTH);
    editor->send(SCI_SETTABWIDTH, 4);

    const QByteArray lexerName = lexerNameForPath(path).toLatin1();
    if (!lexerName.isEmpty()) {
        Scintilla::ILexer5 *lexer = CreateLexer(lexerName.constData());
        if (lexer != nullptr) {
            editor->send(SCI_SETILEXER, 0, reinterpret_cast<Scintilla::sptr_t>(lexer));
            editor->send(SCI_STYLECLEARALL);
            const std::array<std::pair<int, int>, 11> syntaxColors = {{
                {1, 0x8B949E}, {2, 0x8B949E}, {3, 0x8B949E}, {4, 0xD29922},
                {5, 0x79C0FF}, {6, 0xA5D6FF}, {7, 0xA5D6FF}, {8, 0xFF7B72},
                {9, 0xD2A8FF}, {10, 0xD2A8FF}, {11, 0x7EE787},
            }};
            for (const auto &[style, color] : syntaxColors) {
                editor->send(SCI_STYLESETFORE, static_cast<Scintilla::uptr_t>(style), color);
            }
        }
    }
}

} // namespace

MainWindow::MainWindow(QString projectPath, QWidget *parent)
    : QMainWindow(parent), m_projectPath(QFileInfo(projectPath).absoluteFilePath())
{
    setWindowTitle(QStringLiteral("ChxChx Studio — %1").arg(QFileInfo(m_projectPath).fileName()));
    resize(1440, 920);
    buildActions();
    buildLayout();
    statusBar()->showMessage(m_projectPath);
}

void MainWindow::buildActions()
{
    auto *toolbar = addToolBar(QStringLiteral("Archivo"));
    toolbar->setMovable(false);
    toolbar->addAction(makeAction(this, QStringLiteral("Abrir archivo"), QKeySequence::Open, [this] { openFile(); }));
    toolbar->addAction(makeAction(this, QStringLiteral("Guardar"), QKeySequence::Save, [this] { saveFile(); }));
    toolbar->addAction(makeAction(this, QStringLiteral("Guardar como"), QKeySequence::SaveAs, [this] { saveFileAs(); }));
    toolbar->addAction(makeAction(this, QStringLiteral("Buscar en archivo"), QKeySequence(QStringLiteral("Ctrl+F")), [this] {
        findInCurrentFile();
    }));
    toolbar->addAction(makeAction(this, QStringLiteral("Actualizar"), QKeySequence(QStringLiteral("F5")), [this] { refreshArea(); }));
    toolbar->addAction(makeAction(this, QStringLiteral("Paleta de comandos"), QKeySequence(QStringLiteral("Ctrl+P")), [this] {
        openCommandPalette();
    }));
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
    const QString path = m_fileModel->filePath(index);
    if (!m_fileModel->isDir(index)) {
        openPath(path);
    }
}

void MainWindow::openPath(const QString &path)
{
    for (int index = 0; index < m_editorTabs->count(); ++index) {
        auto *editor = qobject_cast<ScintillaEditBase *>(m_editorTabs->widget(index));
        if (editor != nullptr && editor->property("filePath").toString() == path) {
            m_editorTabs->setCurrentIndex(index);
            m_mainPages->setCurrentWidget(m_editorTabs);
            return;
        }
    }

    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        QMessageBox::warning(this, QStringLiteral("No se pudo abrir"), file.errorString());
        return;
    }
    auto *editor = new ScintillaEditBase(m_editorTabs);
    editor->setProperty("filePath", path);
    configureCodeEditor(editor, path);
    setEditorText(editor, QString::fromUtf8(file.readAll()));
    const int tab = m_editorTabs->addTab(editor, QFileInfo(path).fileName());
    m_editorTabs->setTabToolTip(tab, path);
    m_editorTabs->setCurrentIndex(tab);
    m_mainPages->setCurrentWidget(m_editorTabs);
    connect(editor, &ScintillaEditBase::savePointChanged, this, [this, editor](bool dirty) {
        editor->setProperty("modified", dirty);
        const int tabIndex = m_editorTabs->indexOf(editor);
        if (tabIndex >= 0) {
            const QString name = QFileInfo(editor->property("filePath").toString()).fileName();
            m_editorTabs->setTabText(tabIndex, name + (dirty ? QStringLiteral(" •") : QString()));
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
    const int tab = m_editorTabs->currentIndex();
    m_editorTabs->setTabText(tab, QFileInfo(currentFilePath()).fileName());
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

void MainWindow::closeEditorTab(int index)
{
    auto *editor = qobject_cast<ScintillaEditBase *>(m_editorTabs->widget(index));
    if (editor != nullptr && editor->property("modified").toBool()) {
        const auto result = QMessageBox::question(this, QStringLiteral("Cambios sin guardar"),
            QStringLiteral("¿Cerrar esta pestaña y descartar los cambios?"),
            QMessageBox::Discard | QMessageBox::Cancel, QMessageBox::Cancel);
        if (result != QMessageBox::Discard) {
            return;
        }
    }
    QWidget *page = m_editorTabs->widget(index);
    m_editorTabs->removeTab(index);
    page->deleteLater();
}

void MainWindow::selectArea(int row)
{
    if (row < 0 || row >= m_areaList->count()) {
        return;
    }
    const auto *item = m_areaList->item(row);
    m_currentArea = item->data(Qt::UserRole).toString();
    if (m_currentArea == QStringLiteral("handoff")) {
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
        m_mainPages->setCurrentWidget(m_editorTabs);
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
        {QStringLiteral("Guardar archivo"), QStringLiteral("save")},
        {QStringLiteral("Guardar archivo como…"), QStringLiteral("save-as")},
        {QStringLiteral("Buscar en archivo…"), QStringLiteral("find")},
        {QStringLiteral("Actualizar vista actual"), QStringLiteral("refresh")},
        {QStringLiteral("Ejecutar acción principal de la vista"), QStringLiteral("primary")},
        {QStringLiteral("Ejecutar acción secundaria de la vista"), QStringLiteral("secondary")},
        {QStringLiteral("Ejecutar tercera acción de la vista"), QStringLiteral("tertiary")},
        {QStringLiteral("Ejecutar cuarta acción de la vista"), QStringLiteral("quaternary")},
        {QStringLiteral("Abrir una terminal nueva del workspace"), QStringLiteral("workspace-terminal")},
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
        if (schema == BridgeSchemas::ResourcesOverview) {
            output = formatResourcesOverview(payload);
        } else if (schema == BridgeSchemas::ProjectStatus) {
            const QJsonObject workspace = payload.value(QStringLiteral("workspace")).toObject();
            m_projectTrusted = workspace.value(QStringLiteral("trusted")).toBool();
            m_workspaceStatus = workspace.value(QStringLiteral("status")).toString();
            updateAreaTargets(payload, m_commandArea);
            updateAreaActionState();
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
    return qobject_cast<ScintillaEditBase *>(m_editorTabs->currentWidget());
}

QString MainWindow::currentFilePath() const
{
    const auto *editor = currentEditor();
    return editor == nullptr ? QString() : editor->property("filePath").toString();
}
