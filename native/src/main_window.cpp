#include "main_window.hpp"
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

struct Area {
    const char *id;
    const char *label;
    const char *description;
};

constexpr Area areas[] = {
    {"overview", "Inicio", "Estado del proyecto y accesos rápidos."},
    {"project", "Proyecto", "Procesos, confianza y terminales del workspace."},
    {"agents", "Agentes", "Disponibilidad, sesiones, presets y chats."},
    {"processes", "Procesos", "Procesos administrados y consumo."},
    {"projects", "Proyectos", "Proyectos registrados y cambio de contexto."},
    {"skills", "Skills", "Catálogo, recomendaciones y selección por proyecto."},
    {"packs", "Tech Packs", "Packs detectados y composición de skills."},
    {"resources", "Recursos", "RAM, swap, CPU y procesos por proyecto."},
    {"handoff", "Handoff", "Estado transferible entre sesiones de trabajo."},
    {"memory", "Memoria", "Notas persistentes de este proyecto."},
    {"chats", "Chats", "Historial local de conversaciones de agentes."},
    {"errors", "Errores", "Errores recientes registrados por ChxChx."},
    {"setup", "Configuración", "Init, herramientas, MCP y diagnóstico."},
    {"guide", "Guía", "Flujos de trabajo y ayuda contextual."},
    {"brand", "Marca", "Identidad y versión de ChxChx Studio."},
};

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

QString formatBytes(double bytes)
{
    constexpr double kibibyte = 1024.0;
    constexpr double mebibyte = kibibyte * 1024.0;
    constexpr double gibibyte = mebibyte * 1024.0;
    if (bytes >= gibibyte) {
        return QStringLiteral("%1 GiB").arg(bytes / gibibyte, 0, 'f', 1);
    }
    if (bytes >= mebibyte) {
        return QStringLiteral("%1 MiB").arg(bytes / mebibyte, 0, 'f', 0);
    }
    return QStringLiteral("%1 KiB").arg(bytes / kibibyte, 0, 'f', 0);
}

QString displayPercent(const QJsonValue &value)
{
    return value.isDouble()
        ? QStringLiteral("%1%").arg(value.toDouble(), 0, 'f', 0)
        : QStringLiteral("no disponible");
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

void MainWindow::buildLayout()
{
    m_fileModel = new QFileSystemModel(this);
    m_fileModel->setFilter(QDir::AllEntries | QDir::NoDotAndDotDot | QDir::Hidden | QDir::System);
    m_fileModel->setRootPath(m_projectPath);

    m_projectTree = new QTreeView(this);
    m_projectTree->setModel(m_fileModel);
    m_projectTree->setRootIndex(m_fileModel->index(m_projectPath));
    m_projectTree->setHeaderHidden(true);
    for (int column = 1; column < m_fileModel->columnCount(); ++column) {
        m_projectTree->hideColumn(column);
    }
    connect(m_projectTree, &QTreeView::doubleClicked, this, &MainWindow::openTreeFile);

    auto *projectDock = new QDockWidget(QStringLiteral("Proyecto"), this);
    projectDock->setObjectName(QStringLiteral("projectDock"));
    projectDock->setWidget(m_projectTree);
    addDockWidget(Qt::LeftDockWidgetArea, projectDock);

    m_editorTabs = new QTabWidget(this);
    m_editorTabs->setTabsClosable(true);
    m_editorTabs->setMovable(true);
    connect(m_editorTabs, &QTabWidget::tabCloseRequested, this, &MainWindow::closeEditorTab);
    m_mainPages = new QStackedWidget(this);
    m_mainPages->addWidget(m_editorTabs);

    m_handoffPage = new QWidget(m_mainPages);
    auto *handoffLayout = new QVBoxLayout(m_handoffPage);
    auto *handoffForm = new QFormLayout();
    m_handoffSummary = new QLineEdit(QStringLiteral("Estado del workspace actualizado."), m_handoffPage);
    m_handoffPending = new QLineEdit(QStringLiteral("Revisar el siguiente entregable."), m_handoffPage);
    m_handoffValidation = new QLineEdit(QStringLiteral("Ejecutar las pruebas relevantes antes de continuar."), m_handoffPage);
    handoffForm->addRow(QStringLiteral("Resumen"), m_handoffSummary);
    handoffForm->addRow(QStringLiteral("Pendiente"), m_handoffPending);
    handoffForm->addRow(QStringLiteral("Validación"), m_handoffValidation);
    handoffLayout->addLayout(handoffForm);
    handoffLayout->addWidget(new QLabel(QStringLiteral("Handoff actual"), m_handoffPage));
    m_handoffPreview = new QPlainTextEdit(m_handoffPage);
    m_handoffPreview->setReadOnly(true);
    handoffLayout->addWidget(m_handoffPreview, 1);
    m_mainPages->addWidget(m_handoffPage);

    m_memoryPage = new QWidget(m_mainPages);
    auto *memoryLayout = new QHBoxLayout(m_memoryPage);
    auto *memoryListPanel = new QWidget(m_memoryPage);
    auto *memoryListLayout = new QVBoxLayout(memoryListPanel);
    m_memorySearch = new QLineEdit(memoryListPanel);
    m_memorySearch->setPlaceholderText(QStringLiteral("Buscar en títulos y notas…"));
    m_memorySummary = new QLabel(memoryListPanel);
    m_memorySummary->setWordWrap(true);
    m_memoryList = new QListWidget(memoryListPanel);
    memoryListLayout->addWidget(m_memorySearch);
    memoryListLayout->addWidget(m_memorySummary);
    memoryListLayout->addWidget(m_memoryList, 1);
    m_memoryDetail = new QPlainTextEdit(m_memoryPage);
    m_memoryDetail->setReadOnly(true);
    auto *memorySplitter = new QSplitter(Qt::Horizontal, m_memoryPage);
    memorySplitter->addWidget(memoryListPanel);
    memorySplitter->addWidget(m_memoryDetail);
    memorySplitter->setStretchFactor(1, 1);
    memoryLayout->addWidget(memorySplitter);
    m_mainPages->addWidget(m_memoryPage);

    m_chatsPage = new QWidget(m_mainPages);
    auto *chatsLayout = new QHBoxLayout(m_chatsPage);
    auto *chatListPanel = new QWidget(m_chatsPage);
    auto *chatListLayout = new QVBoxLayout(chatListPanel);
    m_chatSearch = new QLineEdit(chatListPanel);
    m_chatSearch->setPlaceholderText(QStringLiteral("Buscar en conversaciones…"));
    m_chatSummary = new QLabel(chatListPanel);
    m_chatSummary->setWordWrap(true);
    m_chatList = new QListWidget(chatListPanel);
    chatListLayout->addWidget(m_chatSearch);
    chatListLayout->addWidget(m_chatSummary);
    chatListLayout->addWidget(m_chatList, 1);
    m_chatDetail = new QPlainTextEdit(m_chatsPage);
    m_chatDetail->setReadOnly(true);
    auto *chatSplitter = new QSplitter(Qt::Horizontal, m_chatsPage);
    chatSplitter->addWidget(chatListPanel);
    chatSplitter->addWidget(m_chatDetail);
    chatSplitter->setStretchFactor(1, 1);
    chatsLayout->addWidget(chatSplitter);
    m_mainPages->addWidget(m_chatsPage);

    m_errorsPage = new QWidget(m_mainPages);
    auto *errorsLayout = new QHBoxLayout(m_errorsPage);
    m_errorsList = new QListWidget(m_errorsPage);
    m_errorsSummary = new QLabel(m_errorsPage);
    m_errorsSummary->setWordWrap(true);
    m_errorDetail = new QPlainTextEdit(m_errorsPage);
    m_errorDetail->setReadOnly(true);
    auto *errorListPanel = new QWidget(m_errorsPage);
    auto *errorListLayout = new QVBoxLayout(errorListPanel);
    errorListLayout->addWidget(m_errorsSummary);
    errorListLayout->addWidget(m_errorsList, 1);
    auto *errorSplitter = new QSplitter(Qt::Horizontal, m_errorsPage);
    errorSplitter->addWidget(errorListPanel);
    errorSplitter->addWidget(m_errorDetail);
    errorSplitter->setStretchFactor(1, 1);
    errorsLayout->addWidget(errorSplitter);
    m_mainPages->addWidget(m_errorsPage);

    m_setupPage = new QWidget(m_mainPages);
    auto *setupLayout = new QVBoxLayout(m_setupPage);
    auto *setupTitle = new QLabel(QStringLiteral("Preparación del proyecto"), m_setupPage);
    QFont setupTitleFont = setupTitle->font();
    setupTitleFont.setPointSize(setupTitleFont.pointSize() + 4);
    setupTitleFont.setBold(true);
    setupTitle->setFont(setupTitleFont);
    setupLayout->addWidget(setupTitle);
    auto *setupDescription = new QLabel(
        QStringLiteral("Las escrituras pasan primero por --dry-run y requieren confirmación. "
                       "El diagnóstico es de solo lectura. Elige una operación en el panel ChxChx."),
        m_setupPage);
    setupDescription->setWordWrap(true);
    setupLayout->addWidget(setupDescription);
    auto *setupSteps = new QLabel(
        QStringLiteral("Preparación completa: instala Basic Memory y Serena, inicializa .ai/ y configura MCP.\n"
                       "Init completo: agrega reglas y archivos administrados al proyecto.\n"
                       "Init mínimo: crea la configuración compacta del proyecto.\n"
                       "Herramientas: instala dependencias base.\n"
                       "MCP: integra los clientes soportados para este proyecto."),
        m_setupPage);
    setupSteps->setWordWrap(true);
    setupSteps->setTextInteractionFlags(Qt::TextSelectableByMouse);
    setupLayout->addWidget(setupSteps);
    auto *setupSafety = new QLabel(QStringLiteral("Las operaciones usan los comandos CLI existentes; "
        "los backups y las reglas de confianza permanecen bajo el control de ChxChx."), m_setupPage);
    setupSafety->setWordWrap(true);
    setupLayout->addWidget(setupSafety);
    setupLayout->addStretch(1);
    m_mainPages->addWidget(m_setupPage);

    m_guidePage = new QWidget(m_mainPages);
    auto *guideLayout = new QVBoxLayout(m_guidePage);
    auto *guideTitle = new QLabel(QStringLiteral("Guía rápida de ChxChx Studio"), m_guidePage);
    QFont guideTitleFont = guideTitle->font();
    guideTitleFont.setPointSize(guideTitleFont.pointSize() + 4);
    guideTitleFont.setBold(true);
    guideTitle->setFont(guideTitleFont);
    guideLayout->addWidget(guideTitle);
    auto *guideText = new QLabel(
        QStringLiteral("• El árbol de la izquierda abre archivos en pestañas.\n"
                       "• El panel ChxChx muestra acciones de la vista seleccionada.\n"
                       "• Actividad conserva la salida reciente de consultas y operaciones.\n"
                       "• La paleta puede abrir una terminal Zellij nueva o adjuntarse al workspace en un emulador externo.\n"
                       "• Las vistas de Memoria, Chats y Errores son de solo lectura.\n"
                       "• Las escrituras muestran una previsualización y piden confirmación.\n\n"
                       "Atajos\n"
                       "Ctrl+O  Abrir archivo\n"
                       "Ctrl+S  Guardar\n"
                       "Ctrl+Shift+S  Guardar como\n"
                       "Ctrl+F  Buscar en el archivo actual\n"
                       "Ctrl+P  Paleta de comandos\n"
                       "F5  Actualizar vista"),
        m_guidePage);
    guideText->setWordWrap(true);
    guideText->setTextInteractionFlags(Qt::TextSelectableByMouse);
    guideLayout->addWidget(guideText);
    guideLayout->addStretch(1);
    m_mainPages->addWidget(m_guidePage);

    m_brandPage = new QWidget(m_mainPages);
    auto *brandLayout = new QVBoxLayout(m_brandPage);
    auto *brandName = new QLabel(QStringLiteral("CHXCHX"), m_brandPage);
    QFont brandFont = brandName->font();
    brandFont.setPointSize(32);
    brandFont.setBold(true);
    brandName->setFont(brandFont);
    brandLayout->addWidget(brandName);
    auto *brandSubtitle = new QLabel(QStringLiteral("TECH LEAD  ·  STUDIO"), m_brandPage);
    brandSubtitle->setStyleSheet(QStringLiteral("letter-spacing: 3px; color: #78a9ff;"));
    brandLayout->addWidget(brandSubtitle);
    auto *brandDescription = new QLabel(
        QStringLiteral("Control plane local para proyectos de desarrollo asistido.\n"
                       "Workspace, agentes, skills, memoria y recursos en una interfaz nativa."),
        m_brandPage);
    brandDescription->setWordWrap(true);
    brandLayout->addWidget(brandDescription);
    brandLayout->addStretch(1);
    m_mainPages->addWidget(m_brandPage);

    setCentralWidget(m_mainPages);
    connect(m_memorySearch, &QLineEdit::returnPressed, this, &MainWindow::refreshArea);
    connect(m_memoryList, &QListWidget::currentItemChanged, this,
            [this](QListWidgetItem *item) { selectMemoryNote(item); });
    connect(m_chatSearch, &QLineEdit::returnPressed, this, &MainWindow::refreshArea);
    connect(m_chatList, &QListWidget::currentItemChanged, this,
            [this](QListWidgetItem *item) { selectChatConversation(item); });
    connect(m_errorsList, &QListWidget::currentItemChanged, this,
            [this](QListWidgetItem *item) {
                m_errorDetail->setPlainText(item == nullptr
                    ? QStringLiteral("Selecciona un error para leer el detalle.")
                    : item->data(Qt::UserRole).toString());
            });

    auto *control = new QWidget(this);
    auto *controlLayout = new QVBoxLayout(control);
    m_areaList = new QListWidget(control);
    m_areaDescription = new QLabel(control);
    m_areaDescription->setWordWrap(true);
    for (const auto &area : areas) {
        auto *item = new QListWidgetItem(QString::fromUtf8(area.label), m_areaList);
        item->setData(Qt::UserRole, QString::fromUtf8(area.id));
    }
    m_areaList->setCurrentRow(0);
    controlLayout->addWidget(m_areaList);
    controlLayout->addWidget(m_areaDescription);
    m_targetSelector = new QComboBox(control);
    m_targetSelector->setPlaceholderText(QStringLiteral("Selecciona un elemento"));
    m_primaryAction = new QPushButton(control);
    m_secondaryAction = new QPushButton(control);
    m_tertiaryAction = new QPushButton(control);
    m_quaternaryAction = new QPushButton(control);
    controlLayout->addWidget(m_targetSelector);
    controlLayout->addWidget(m_primaryAction);
    controlLayout->addWidget(m_secondaryAction);
    controlLayout->addWidget(m_tertiaryAction);
    controlLayout->addWidget(m_quaternaryAction);
    connect(m_areaList, &QListWidget::currentRowChanged, this, &MainWindow::selectArea);
    connect(m_targetSelector, &QComboBox::currentTextChanged, this, [this] { updateAreaActionState(); });
    connect(m_primaryAction, &QPushButton::clicked, this, &MainWindow::performPrimaryAreaAction);
    connect(m_secondaryAction, &QPushButton::clicked, this, &MainWindow::performSecondaryAreaAction);
    connect(m_tertiaryAction, &QPushButton::clicked, this, &MainWindow::performTertiaryAreaAction);
    connect(m_quaternaryAction, &QPushButton::clicked, this, &MainWindow::performQuaternaryAreaAction);

    auto *controlDock = new QDockWidget(QStringLiteral("ChxChx"), this);
    controlDock->setObjectName(QStringLiteral("controlDock"));
    controlDock->setWidget(control);
    addDockWidget(Qt::RightDockWidgetArea, controlDock);

    m_output = new QPlainTextEdit(this);
    m_output->setReadOnly(true);
    m_output->setMaximumBlockCount(4000);
    auto *outputDock = new QDockWidget(QStringLiteral("Actividad"), this);
    outputDock->setObjectName(QStringLiteral("activityDock"));
    outputDock->setWidget(m_output);
    addDockWidget(Qt::BottomDockWidgetArea, outputDock);

    m_command = new QProcess(this);
    m_command->setWorkingDirectory(m_projectPath);
    m_command->setProcessChannelMode(QProcess::SeparateChannels);
    connect(m_command, qOverload<int, QProcess::ExitStatus>(&QProcess::finished),
            this, &MainWindow::finishCommand);
    selectArea(0);
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
    if (m_command->state() != QProcess::NotRunning) {
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
    if (m_command->state() != QProcess::NotRunning) {
        statusBar()->showMessage(QStringLiteral("Espera a que termine el comando actual."));
        return;
    }
    m_commandArea = m_currentArea;
    m_command->setProgram(QStringLiteral("chxchx-tech"));
    m_command->setArguments(arguments);
    m_command->start();
    statusBar()->showMessage(QStringLiteral("Consultando ChxChx…"));
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
    QString output = QString::fromUtf8(m_command->readAllStandardOutput()).trimmed();
    const QString error = QString::fromUtf8(m_command->readAllStandardError()).trimmed();
    const auto json = QJsonDocument::fromJson(output.toUtf8());
    if (json.isObject()) {
        const QJsonObject payload = json.object();
        if (payload.value(QStringLiteral("schema")).toString() == QStringLiteral("chxchx.resources-overview")) {
            output = formatResourcesOverview(payload);
        } else if (payload.value(QStringLiteral("schema")).toString() == QStringLiteral("chxchx.project-status")) {
            const QJsonObject workspace = payload.value(QStringLiteral("workspace")).toObject();
            m_projectTrusted = workspace.value(QStringLiteral("trusted")).toBool();
            m_workspaceStatus = workspace.value(QStringLiteral("status")).toString();
            updateAreaTargets(payload, m_commandArea);
            updateAreaActionState();
            output = formatBridgeStatus(payload, m_commandArea);
        } else if (payload.value(QStringLiteral("schema")).toString() == QStringLiteral("chxchx.handoff")) {
            showHandoff(payload);
            output = payload.value(QStringLiteral("exists")).toBool()
                ? QStringLiteral("Handoff cargado en modo lectura.")
                : QStringLiteral("Este proyecto todavía no tiene handoff.");
        } else if (payload.value(QStringLiteral("schema")).toString() == QStringLiteral("chxchx.memory")) {
            showMemory(payload);
            output = QStringLiteral("Memoria local actualizada: %1 nota(s).")
                .arg(payload.value(QStringLiteral("notes")).toArray().size());
        } else if (payload.value(QStringLiteral("schema")).toString() == QStringLiteral("chxchx.conversations")) {
            showConversations(payload);
            output = QStringLiteral("Conversaciones locales: %1.")
                .arg(payload.value(QStringLiteral("conversations")).toArray().size());
        } else if (payload.value(QStringLiteral("schema")).toString() == QStringLiteral("chxchx.conversation")) {
            showConversation(payload);
            output = QStringLiteral("Conversación cargada en solo lectura.");
        } else if (payload.value(QStringLiteral("schema")).toString() == QStringLiteral("chxchx.errors")) {
            showErrors(payload);
            output = QStringLiteral("Errores locales: %1.")
                .arg(payload.value(QStringLiteral("errors")).toArray().size());
        } else if (payload.value(QStringLiteral("schema")).toString() == QStringLiteral("chxchx.error")) {
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

void MainWindow::configureAreaActions()
{
    m_targetSelector->clear();
    const bool agents = m_currentArea == QStringLiteral("agents");
    const bool processes = m_currentArea == QStringLiteral("processes");
    const bool project = m_currentArea == QStringLiteral("project");
    const bool projects = m_currentArea == QStringLiteral("projects");
    const bool skills = m_currentArea == QStringLiteral("skills");
    const bool packs = m_currentArea == QStringLiteral("packs");
    const bool handoff = m_currentArea == QStringLiteral("handoff");
    const bool memory = m_currentArea == QStringLiteral("memory");
    const bool chats = m_currentArea == QStringLiteral("chats");
    const bool errors = m_currentArea == QStringLiteral("errors");
    const bool setup = m_currentArea == QStringLiteral("setup");
    const bool supported = agents || processes || projects || skills || packs;
    m_targetSelector->setVisible(supported || setup);
    m_primaryAction->setVisible(supported || project || handoff || memory || chats || errors || setup);
    m_secondaryAction->setVisible(agents || processes || project || projects || skills || packs || handoff || setup);
    m_tertiaryAction->setVisible(agents || project || packs);
    m_quaternaryAction->setVisible(project);
    if (agents) {
        m_primaryAction->setText(QStringLiteral("Iniciar agente"));
        m_secondaryAction->setText(QStringLiteral("Nuevo chat con contexto"));
        m_tertiaryAction->setText(QStringLiteral("Iniciar todos"));
    } else if (processes) {
        m_primaryAction->setText(QStringLiteral("Iniciar proceso"));
        m_secondaryAction->setText(QStringLiteral("Detener proceso"));
    } else if (project) {
        m_primaryAction->setText(QStringLiteral("Confiar proyecto"));
        m_secondaryAction->setText(QStringLiteral("Iniciar workspace"));
        m_tertiaryAction->setText(QStringLiteral("Suspender workspace"));
        m_quaternaryAction->setText(QStringLiteral("Detener procesos"));
    } else if (projects) {
        m_primaryAction->setText(QStringLiteral("Cambiar a proyecto"));
        m_secondaryAction->setText(QStringLiteral("Confiar proyecto elegido"));
    } else if (skills) {
        m_primaryAction->setText(QStringLiteral("Habilitar skill"));
        m_secondaryAction->setText(QStringLiteral("Sincronizar contexto"));
    } else if (packs) {
        m_primaryAction->setText(QStringLiteral("Aplicar Tech Pack"));
        m_secondaryAction->setText(QStringLiteral("Aplicar packs detectados"));
        m_tertiaryAction->setText(QStringLiteral("Sincronizar contexto"));
    } else if (handoff) {
        m_primaryAction->setText(QStringLiteral("Actualizar handoff"));
        m_secondaryAction->setText(QStringLiteral("Recargar handoff"));
    } else if (memory) {
        m_primaryAction->setText(QStringLiteral("Buscar / actualizar notas"));
    } else if (chats) {
        m_primaryAction->setText(QStringLiteral("Buscar conversaciones"));
    } else if (errors) {
        m_primaryAction->setText(QStringLiteral("Actualizar errores"));
    } else if (setup) {
        m_targetSelector->addItem(QStringLiteral("Preparación completa"), QStringLiteral("setup"));
        m_targetSelector->addItem(QStringLiteral("Init completo"), QStringLiteral("init"));
        m_targetSelector->addItem(QStringLiteral("Init mínimo"), QStringLiteral("init-minimal"));
        m_targetSelector->addItem(QStringLiteral("Instalar herramientas base"), QStringLiteral("install"));
        m_targetSelector->addItem(QStringLiteral("Configurar integraciones MCP"), QStringLiteral("integrate"));
        m_primaryAction->setText(QStringLiteral("Previsualizar y ejecutar"));
        m_secondaryAction->setText(QStringLiteral("Ejecutar diagnóstico"));
    }
    updateAreaActionState();
}

void MainWindow::updateAreaActionState()
{
    const bool hasSelection = m_targetSelector != nullptr && m_targetSelector->currentIndex() >= 0;
    m_primaryAction->setEnabled(hasSelection);
    m_secondaryAction->setEnabled(hasSelection);
    m_tertiaryAction->setEnabled(true);
    m_quaternaryAction->setEnabled(true);
    if (m_currentArea == QStringLiteral("processes") && hasSelection) {
        const QString processStatus = m_targetSelector->currentData().toString();
        m_primaryAction->setEnabled(processStatus != QStringLiteral("RUNNING") && processStatus != QStringLiteral("UNKNOWN"));
        m_secondaryAction->setEnabled(processStatus == QStringLiteral("RUNNING"));
    }
    if (m_currentArea == QStringLiteral("agents")) {
        m_tertiaryAction->setEnabled(m_targetSelector->count() > 0);
    }
    if (m_currentArea == QStringLiteral("skills") && hasSelection) {
        const bool enabled = m_targetSelector->currentData(Qt::UserRole + 1).toBool();
        m_primaryAction->setText(enabled ? QStringLiteral("Deshabilitar skill") : QStringLiteral("Habilitar skill"));
        m_primaryAction->setEnabled(true);
        m_secondaryAction->setEnabled(true);
    }
    if (m_currentArea == QStringLiteral("handoff")) {
        m_primaryAction->setEnabled(true);
        m_secondaryAction->setEnabled(true);
    }
    if (m_currentArea == QStringLiteral("memory") || m_currentArea == QStringLiteral("chats")
        || m_currentArea == QStringLiteral("errors")) {
        m_primaryAction->setEnabled(true);
    }
    if (m_currentArea == QStringLiteral("setup")) {
        m_primaryAction->setEnabled(m_targetSelector->currentIndex() >= 0);
        m_secondaryAction->setEnabled(true);
    }
    if (m_currentArea == QStringLiteral("packs")) {
        const bool hasPack = hasSelection;
        bool hasDetected = false;
        for (int index = 0; index < m_targetSelector->count(); ++index) {
            hasDetected = hasDetected || m_targetSelector->itemData(index, Qt::UserRole + 2).toBool();
        }
        m_primaryAction->setEnabled(hasPack);
        m_secondaryAction->setEnabled(hasDetected);
        m_tertiaryAction->setEnabled(true);
    }
    if (m_currentArea == QStringLiteral("project")) {
        m_primaryAction->setText(m_projectTrusted
            ? QStringLiteral("Proyecto confiable") : QStringLiteral("Confiar proyecto"));
        m_primaryAction->setEnabled(!m_projectTrusted);
        const bool suspended = m_workspaceStatus == QStringLiteral("SUSPENDED");
        m_secondaryAction->setText(suspended
            ? QStringLiteral("Reanudar workspace") : QStringLiteral("Iniciar workspace"));
        m_secondaryAction->setEnabled(m_projectTrusted && m_workspaceStatus != QStringLiteral("ACTIVE"));
        m_tertiaryAction->setEnabled(m_workspaceStatus == QStringLiteral("ACTIVE"));
        m_quaternaryAction->setEnabled(m_workspaceStatus == QStringLiteral("ACTIVE")
            || m_workspaceStatus == QStringLiteral("ERROR"));
    }
    if (m_currentArea == QStringLiteral("projects") && hasSelection) {
        const bool sameProject = m_targetSelector->currentData().toString() == m_projectPath;
        const bool trusted = m_targetSelector->currentData(Qt::UserRole + 2).toBool();
        m_primaryAction->setEnabled(!sameProject && trusted);
        m_secondaryAction->setText(trusted
            ? QStringLiteral("Proyecto confiable") : QStringLiteral("Confiar proyecto elegido"));
        m_secondaryAction->setEnabled(!trusted);
    }
}

void MainWindow::updateAreaTargets(const QJsonObject &payload, const QString &area)
{
    if (area != m_currentArea || (area != QStringLiteral("agents")
        && area != QStringLiteral("processes") && area != QStringLiteral("projects")
        && area != QStringLiteral("skills") && area != QStringLiteral("packs"))) {
        return;
    }
    const QString selected = area == QStringLiteral("projects")
        ? m_targetSelector->currentData(Qt::UserRole + 1).toString()
        : (area == QStringLiteral("skills") || area == QStringLiteral("packs")
            ? m_targetSelector->currentData().toString()
            : m_targetSelector->currentText());
    m_targetSelector->clear();
    if (area == QStringLiteral("skills") || area == QStringLiteral("packs")) {
        const QString key = area == QStringLiteral("skills") ? QStringLiteral("skills") : QStringLiteral("packs");
        for (const auto &value : payload.value(key).toArray()) {
            const auto item = value.toObject();
            const QString name = item.value(QStringLiteral("name")).toString();
            if (name.isEmpty()) {
                continue;
            }
            const bool enabled = item.value(QStringLiteral("enabled")).toBool();
            const bool recommended = item.value(QStringLiteral("recommended")).toBool();
            QString label = name;
            if (recommended) {
                label += QStringLiteral(" · recomendada");
            }
            if (area == QStringLiteral("skills")) {
                label += enabled ? QStringLiteral(" · habilitada") : QStringLiteral(" · deshabilitada");
            }
            m_targetSelector->addItem(label, name);
            m_targetSelector->setItemData(m_targetSelector->count() - 1, enabled, Qt::UserRole + 1);
            m_targetSelector->setItemData(m_targetSelector->count() - 1, recommended, Qt::UserRole + 2);
        }
        const int previous = m_targetSelector->findData(selected);
        if (previous >= 0) {
            m_targetSelector->setCurrentIndex(previous);
        }
        updateAreaActionState();
        return;
    }
    if (area == QStringLiteral("projects")) {
        for (const auto &value : payload.value(QStringLiteral("registered_projects")).toArray()) {
            const auto project = value.toObject();
            const QString alias = project.value(QStringLiteral("alias")).toString();
            if (alias.isEmpty() || !project.value(QStringLiteral("exists")).toBool()) {
                continue;
            }
            const QString trustLabel = project.value(QStringLiteral("trusted")).toBool()
                ? QStringLiteral("confiable") : QStringLiteral("requiere trust");
            const QString label = QStringLiteral("%1 · %2 · %3 · %4")
                .arg(alias, project.value(QStringLiteral("name")).toString(),
                     project.value(QStringLiteral("status")).toString(), trustLabel);
            const QString path = project.value(QStringLiteral("path")).toString();
            m_targetSelector->addItem(label, path);
            const int index = m_targetSelector->count() - 1;
            m_targetSelector->setItemData(index, alias, Qt::UserRole + 1);
            m_targetSelector->setItemData(index, project.value(QStringLiteral("trusted")).toBool(), Qt::UserRole + 2);
            if (path == m_projectPath) {
                m_targetSelector->setCurrentIndex(index);
            }
        }
        const int previous = m_targetSelector->findData(selected, Qt::UserRole + 1);
        if (previous >= 0) {
            m_targetSelector->setCurrentIndex(previous);
        }
        updateAreaActionState();
        return;
    }
    const QString key = area == QStringLiteral("agents") ? QStringLiteral("agents") : QStringLiteral("processes");
    for (const auto &value : payload.value(key).toArray()) {
        const auto item = value.toObject();
        const QString id = item.value(QStringLiteral("id")).toString();
        if (!id.isEmpty()) {
            const QString status = item.value(QStringLiteral("status")).toString();
            m_targetSelector->addItem(id, status);
        }
    }
    const int previous = m_targetSelector->findText(selected);
    if (previous >= 0) {
        m_targetSelector->setCurrentIndex(previous);
    }
    updateAreaActionState();
}

void MainWindow::setProjectRoot(const QString &path)
{
    m_projectPath = QFileInfo(path).absoluteFilePath();
    m_command->setWorkingDirectory(m_projectPath);
    m_fileModel->setRootPath(m_projectPath);
    m_projectTree->setRootIndex(m_fileModel->index(m_projectPath));
    setWindowTitle(QStringLiteral("ChxChx Studio — %1").arg(QFileInfo(m_projectPath).fileName()));
    statusBar()->showMessage(QStringLiteral("Proyecto activo: %1").arg(m_projectPath), 5000);
}

void MainWindow::showHandoff(const QJsonObject &payload)
{
    if (!payload.value(QStringLiteral("exists")).toBool()) {
        m_handoffPreview->setPlainText(QStringLiteral("Todavía no existe .ai/HANDOFF.md en este proyecto."));
        return;
    }
    m_handoffSummary->setText(payload.value(QStringLiteral("summary")).toString());
    m_handoffPending->setText(payload.value(QStringLiteral("pending")).toString());
    m_handoffValidation->setText(payload.value(QStringLiteral("validation")).toString());
    m_handoffPreview->setPlainText(payload.value(QStringLiteral("content")).toString());
}

void MainWindow::showMemory(const QJsonObject &payload)
{
    m_memoryList->clear();
    const QJsonArray notes = payload.value(QStringLiteral("notes")).toArray();
    for (const auto &value : notes) {
        const auto note = value.toObject();
        const QString title = note.value(QStringLiteral("title")).toString();
        const QString modified = note.value(QStringLiteral("modified_at")).toString();
        auto *item = new QListWidgetItem(QStringLiteral("%1 · %2").arg(title, modified), m_memoryList);
        item->setData(Qt::UserRole, note.value(QStringLiteral("content")).toString());
        item->setToolTip(note.value(QStringLiteral("path")).toString());
    }
    const QString query = payload.value(QStringLiteral("query")).toString();
    if (notes.isEmpty()) {
        m_memorySummary->setText(query.isEmpty()
            ? QStringLiteral("No hay notas locales en .ai/memory.")
            : QStringLiteral("No hay notas que coincidan con la búsqueda."));
        m_memoryDetail->clear();
    } else {
        m_memorySummary->setText(QStringLiteral("%1 nota(s) · solo lectura%2")
            .arg(QString::number(notes.size()), query.isEmpty()
                ? QString() : QStringLiteral(" · filtro: %1").arg(query)));
        m_memoryList->setCurrentRow(0);
    }
}

void MainWindow::selectMemoryNote(QListWidgetItem *item)
{
    m_memoryDetail->setPlainText(item == nullptr
        ? QStringLiteral("Selecciona una nota para leerla.")
        : item->data(Qt::UserRole).toString());
}

void MainWindow::showConversations(const QJsonObject &payload)
{
    m_chatList->clear();
    const QJsonArray conversations = payload.value(QStringLiteral("conversations")).toArray();
    for (const auto &value : conversations) {
        const auto conversation = value.toObject();
        const QString provider = conversation.value(QStringLiteral("provider")).toString();
        const QString title = conversation.value(QStringLiteral("title")).toString();
        const QString modified = conversation.value(QStringLiteral("modified_at")).toString();
        auto *item = new QListWidgetItem(QStringLiteral("%1 · %2 · %3").arg(provider, modified, title), m_chatList);
        item->setData(Qt::UserRole, provider);
        item->setData(Qt::UserRole + 1, conversation.value(QStringLiteral("session_id")).toString());
        item->setData(Qt::UserRole + 2, conversation.value(QStringLiteral("preview")).toString());
        item->setToolTip(conversation.value(QStringLiteral("transcript_path")).toString());
    }
    const QString query = payload.value(QStringLiteral("query")).toString();
    m_chatSummary->setText(conversations.isEmpty()
        ? (query.isEmpty()
            ? QStringLiteral("No hay conversaciones locales para este proyecto.")
            : QStringLiteral("No hay conversaciones que coincidan con la búsqueda."))
        : QStringLiteral("%1 conversaciones · Codex y Claude Code · solo lectura")
            .arg(conversations.size()));
    m_chatDetail->clear();
    if (!conversations.isEmpty()) {
        m_chatDetail->setPlainText(QStringLiteral("Selecciona una conversación para leerla."));
    }
}

void MainWindow::showConversation(const QJsonObject &payload)
{
    QStringList lines;
    lines << QStringLiteral("%1 · %2 · %3 · solo lectura")
        .arg(payload.value(QStringLiteral("provider")).toString(),
             payload.value(QStringLiteral("title")).toString(),
             payload.value(QStringLiteral("modified_at")).toString());
    lines << payload.value(QStringLiteral("transcript_path")).toString() << QString();
    const QString provider = payload.value(QStringLiteral("provider")).toString();
    for (const auto &value : payload.value(QStringLiteral("messages")).toArray()) {
        const auto message = value.toObject();
        const QString role = message.value(QStringLiteral("role")).toString();
        lines << QStringLiteral("%1:\n%2")
            .arg(role == QStringLiteral("user") ? QStringLiteral("Tú") : provider,
                 message.value(QStringLiteral("text")).toString());
    }
    m_chatDetail->setPlainText(lines.join(QStringLiteral("\n\n")));
}

void MainWindow::showErrors(const QJsonObject &payload)
{
    m_errorsList->clear();
    const QJsonArray errors = payload.value(QStringLiteral("errors")).toArray();
    for (const auto &value : errors) {
        const auto error = value.toObject();
        const QString date = error.value(QStringLiteral("occurred_at")).toString();
        const QString operation = error.value(QStringLiteral("operation")).toString();
        const QString message = error.value(QStringLiteral("message")).toString();
        auto *item = new QListWidgetItem(QStringLiteral("%1 · %2 · %3")
            .arg(date.left(19).replace(QLatin1Char('T'), QLatin1Char(' ')), operation,
                 message.simplified().left(120)), m_errorsList);
        item->setData(Qt::UserRole, QStringLiteral("%1\nProyecto: %2 · %3\nFecha UTC: %4\n\n%5")
            .arg(operation,
                 error.value(QStringLiteral("project")).toString(),
                 error.value(QStringLiteral("project_path")).toString(),
                 date,
                 message));
    }
    m_errorsSummary->setText(errors.isEmpty()
        ? QStringLiteral("No hay errores guardados para este proyecto.")
        : QStringLiteral("%1 errores recientes · caché local %2")
            .arg(QString::number(errors.size()), payload.value(QStringLiteral("cache_path")).toString()));
    m_errorDetail->setPlainText(QStringLiteral("Selecciona un error para leer el detalle."));
}

void MainWindow::selectChatConversation(QListWidgetItem *item)
{
    if (item == nullptr) {
        return;
    }
    if (m_command->state() != QProcess::NotRunning) {
        statusBar()->showMessage(QStringLiteral("Espera a que termine la consulta actual."));
        return;
    }
    const QStringList arguments = {
        QStringLiteral("bridge"), QStringLiteral("conversation"), m_projectPath,
        item->data(Qt::UserRole).toString(), item->data(Qt::UserRole + 1).toString()};
    runCommand(arguments);
}

void MainWindow::performPrimaryAreaAction()
{
    const QString id = m_targetSelector->currentText();
    if (m_currentArea == QStringLiteral("agents")) {
        const QStringList preview = {QStringLiteral("agent"), QStringLiteral("start"), id,
            QStringLiteral("--path"), m_projectPath, QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("agent"), QStringLiteral("start"), id,
            QStringLiteral("--path"), m_projectPath};
        QStringList forceAction = action;
        forceAction << QStringLiteral("--force");
        runPreview(preview, action, forceAction, QStringLiteral("Confirmar inicio de agente"));
    } else if (m_currentArea == QStringLiteral("processes")) {
        const QStringList preview = {QStringLiteral("process"), QStringLiteral("start"), id,
            QStringLiteral("--path"), m_projectPath, QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("process"), QStringLiteral("start"), id,
            QStringLiteral("--path"), m_projectPath};
        runPreview(preview, action, {}, QStringLiteral("Confirmar inicio de proceso"));
    } else if (m_currentArea == QStringLiteral("project")) {
        const QStringList preview = {QStringLiteral("workspace"), QStringLiteral("trust"), m_projectPath,
            QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("workspace"), QStringLiteral("trust"), m_projectPath};
        runPreview(preview, action, {}, QStringLiteral("Confirmar confianza del proyecto"));
    } else if (m_currentArea == QStringLiteral("projects")) {
        const QString alias = m_targetSelector->currentData(Qt::UserRole + 1).toString();
        const QString path = m_targetSelector->currentData().toString();
        const QStringList preview = {QStringLiteral("projects"), QStringLiteral("switch"), alias,
            QStringLiteral("--dry-run"), QStringLiteral("--no-attach")};
        const QStringList action = {QStringLiteral("projects"), QStringLiteral("switch"), alias,
            QStringLiteral("--no-attach")};
        QStringList forceAction = action;
        forceAction << QStringLiteral("--force");
        if (runPreview(preview, action, forceAction, QStringLiteral("Cambiar de proyecto"))) {
            m_pendingProjectPath = path;
        }
    } else if (m_currentArea == QStringLiteral("skills")) {
        const QString name = m_targetSelector->currentData().toString();
        const bool enabled = m_targetSelector->currentData(Qt::UserRole + 1).toBool();
        const QString operation = enabled ? QStringLiteral("disable") : QStringLiteral("enable");
        const QStringList preview = {QStringLiteral("skill"), operation, name, m_projectPath, QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("skill"), operation, name, m_projectPath};
        runPreview(preview, action, {}, enabled
            ? QStringLiteral("Deshabilitar skill") : QStringLiteral("Habilitar skill"));
    } else if (m_currentArea == QStringLiteral("packs")) {
        const QString name = m_targetSelector->currentData().toString();
        const QStringList preview = {QStringLiteral("pack"), QStringLiteral("apply"), name,
            m_projectPath, QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("pack"), QStringLiteral("apply"), name, m_projectPath};
        runPreview(preview, action, {}, QStringLiteral("Aplicar Tech Pack"));
    } else if (m_currentArea == QStringLiteral("handoff")) {
        if (m_handoffSummary->text().trimmed().isEmpty()
            || m_handoffPending->text().trimmed().isEmpty()
            || m_handoffValidation->text().trimmed().isEmpty()) {
            QMessageBox::warning(this, QStringLiteral("Handoff incompleto"),
                QStringLiteral("Completa resumen, pendiente y validación antes de guardar."));
            return;
        }
        const QStringList base = {QStringLiteral("workspace"), QStringLiteral("handoff"), m_projectPath,
            QStringLiteral("--summary"), m_handoffSummary->text(),
            QStringLiteral("--pending"), m_handoffPending->text(),
            QStringLiteral("--validation"), m_handoffValidation->text()};
        QStringList preview = base;
        preview << QStringLiteral("--dry-run");
        runPreview(preview, base, {}, QStringLiteral("Actualizar handoff"));
    } else if (m_currentArea == QStringLiteral("memory")) {
        refreshArea();
    } else if (m_currentArea == QStringLiteral("chats") || m_currentArea == QStringLiteral("errors")) {
        refreshArea();
    } else if (m_currentArea == QStringLiteral("setup")) {
        const QString operation = m_targetSelector->currentData().toString();
        QStringList preview;
        QStringList action;
        if (operation == QStringLiteral("setup")) {
            preview = {QStringLiteral("setup"), m_projectPath, QStringLiteral("--dry-run")};
            action = {QStringLiteral("setup"), m_projectPath};
        } else if (operation == QStringLiteral("init") || operation == QStringLiteral("init-minimal")) {
            preview = {QStringLiteral("init"), m_projectPath, QStringLiteral("--dry-run")};
            action = {QStringLiteral("init"), m_projectPath};
            if (operation == QStringLiteral("init-minimal")) {
                preview << QStringLiteral("--minimal");
                action << QStringLiteral("--minimal");
            }
        } else if (operation == QStringLiteral("install")) {
            preview = {QStringLiteral("install"), QStringLiteral("--dry-run")};
            action = {QStringLiteral("install")};
        } else if (operation == QStringLiteral("integrate")) {
            preview = {QStringLiteral("integrate"), m_projectPath, QStringLiteral("--client"),
                QStringLiteral("all"), QStringLiteral("--dry-run")};
            action = {QStringLiteral("integrate"), m_projectPath, QStringLiteral("--client"), QStringLiteral("all")};
        }
        runPreview(preview, action, {}, QStringLiteral("Preparar proyecto"));
    }
}

void MainWindow::performSecondaryAreaAction()
{
    if (m_currentArea == QStringLiteral("setup")) {
        runCommand({QStringLiteral("doctor"), m_projectPath});
        return;
    }
    if (m_currentArea == QStringLiteral("handoff")) {
        refreshArea();
        return;
    }
    const QString id = m_targetSelector->currentText();
    if (m_currentArea == QStringLiteral("agents")) {
        const QStringList preview = {QStringLiteral("agent"), QStringLiteral("start"), id,
            QStringLiteral("--path"), m_projectPath, QStringLiteral("--new-chat"), QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("agent"), QStringLiteral("start"), id,
            QStringLiteral("--path"), m_projectPath, QStringLiteral("--new-chat")};
        QStringList forceAction = action;
        forceAction << QStringLiteral("--force");
        runPreview(preview, action, forceAction, QStringLiteral("Confirmar nuevo chat con contexto"));
    } else if (m_currentArea == QStringLiteral("processes")) {
        const QStringList preview = {QStringLiteral("process"), QStringLiteral("stop"), id,
            QStringLiteral("--path"), m_projectPath, QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("process"), QStringLiteral("stop"), id,
            QStringLiteral("--path"), m_projectPath};
        runPreview(preview, action, {}, QStringLiteral("Confirmar detención de proceso"));
    } else if (m_currentArea == QStringLiteral("projects")) {
        const QString path = m_targetSelector->currentData().toString();
        const QStringList preview = {QStringLiteral("workspace"), QStringLiteral("trust"), path,
            QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("workspace"), QStringLiteral("trust"), path};
        runPreview(preview, action, {}, QStringLiteral("Confiar proyecto seleccionado"));
    } else if (m_currentArea == QStringLiteral("project")) {
        const QString command = m_workspaceStatus == QStringLiteral("SUSPENDED")
            ? QStringLiteral("resume") : QStringLiteral("start");
        const QStringList preview = {QStringLiteral("workspace"), command, m_projectPath,
            QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("workspace"), command, m_projectPath};
        const QString title = command == QStringLiteral("resume")
            ? QStringLiteral("Confirmar reanudación del workspace")
            : QStringLiteral("Confirmar inicio del workspace");
        runPreview(preview, action, {}, title);
    } else if (m_currentArea == QStringLiteral("skills")) {
        const QStringList preview = {QStringLiteral("skill"), QStringLiteral("sync"), m_projectPath,
            QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("skill"), QStringLiteral("sync"), m_projectPath};
        runPreview(preview, action, {}, QStringLiteral("Sincronizar contexto de skills"));
    } else if (m_currentArea == QStringLiteral("packs")) {
        const QStringList preview = {QStringLiteral("pack"), QStringLiteral("apply-detected"), m_projectPath,
            QStringLiteral("--dry-run")};
        const QStringList action = {QStringLiteral("pack"), QStringLiteral("apply-detected"), m_projectPath};
        runPreview(preview, action, {}, QStringLiteral("Aplicar Tech Packs detectados"));
    }
}

void MainWindow::performTertiaryAreaAction()
{
    if (m_currentArea != QStringLiteral("agents")) {
        if (m_currentArea == QStringLiteral("project")) {
            const QStringList preview = {QStringLiteral("workspace"), QStringLiteral("suspend"), m_projectPath,
                QStringLiteral("--dry-run")};
            const QStringList action = {QStringLiteral("workspace"), QStringLiteral("suspend"), m_projectPath};
            runPreview(preview, action, {}, QStringLiteral("Confirmar suspensión del workspace"));
        } else if (m_currentArea == QStringLiteral("packs")) {
            const QStringList preview = {QStringLiteral("skill"), QStringLiteral("sync"), m_projectPath,
                QStringLiteral("--dry-run")};
            const QStringList action = {QStringLiteral("skill"), QStringLiteral("sync"), m_projectPath};
            runPreview(preview, action, {}, QStringLiteral("Sincronizar contexto de skills"));
        }
        return;
    }
    const QStringList preview = {QStringLiteral("agent"), QStringLiteral("start"),
        QStringLiteral("--all"), QStringLiteral("--path"), m_projectPath, QStringLiteral("--dry-run")};
    const QStringList action = {QStringLiteral("agent"), QStringLiteral("start"),
        QStringLiteral("--all"), QStringLiteral("--path"), m_projectPath};
    QStringList forceAction = action;
    forceAction << QStringLiteral("--force");
    runPreview(preview, action, forceAction, QStringLiteral("Confirmar inicio de todos los agentes"));
}

void MainWindow::performQuaternaryAreaAction()
{
    if (m_currentArea != QStringLiteral("project")) {
        return;
    }
    const QStringList preview = {QStringLiteral("workspace"), QStringLiteral("stop"), m_projectPath,
        QStringLiteral("--dry-run")};
    const QStringList action = {QStringLiteral("workspace"), QStringLiteral("stop"), m_projectPath};
    runPreview(preview, action, {}, QStringLiteral("Confirmar detención de procesos"));
}

void MainWindow::openNewWorkspaceTerminal()
{
    const QStringList preview = {QStringLiteral("workspace"), QStringLiteral("terminal"),
        m_projectPath, QStringLiteral("--dry-run")};
    const QStringList action = {QStringLiteral("__launch_terminal__"),
        QStringLiteral("chxchx-tech"), QStringLiteral("workspace"),
        QStringLiteral("terminal"), m_projectPath};
    runPreview(preview, action, {}, QStringLiteral("Abrir una terminal nueva"));
}

void MainWindow::attachWorkspaceTerminal()
{
    const QStringList preview = {QStringLiteral("workspace"), QStringLiteral("attach"),
        m_projectPath, QStringLiteral("--dry-run")};
    const QStringList action = {QStringLiteral("__launch_terminal__"),
        QStringLiteral("chxchx-tech"), QStringLiteral("workspace"),
        QStringLiteral("attach"), m_projectPath};
    runPreview(preview, action, {}, QStringLiteral("Adjuntar al workspace"));
}

void MainWindow::attachAgentTerminal()
{
    if (m_currentArea != QStringLiteral("agents") || m_targetSelector->currentIndex() < 0) {
        QMessageBox::information(this, QStringLiteral("Selecciona un agente"),
            QStringLiteral("Abre Agentes y selecciona el agente que quieres adjuntar."));
        return;
    }
    const QString agentId = m_targetSelector->currentText();
    const QStringList preview = {QStringLiteral("agent"), QStringLiteral("attach"), agentId,
        QStringLiteral("--path"), m_projectPath, QStringLiteral("--dry-run")};
    const QStringList action = {QStringLiteral("__launch_terminal__"),
        QStringLiteral("chxchx-tech"), QStringLiteral("agent"), QStringLiteral("attach"), agentId,
        QStringLiteral("--path"), m_projectPath};
    runPreview(preview, action, {}, QStringLiteral("Adjuntar al agente seleccionado"));
}

bool MainWindow::runPreview(
    const QStringList &previewArguments,
    const QStringList &actionArguments,
    const QStringList &forceArguments,
    const QString &title)
{
    if (m_command->state() != QProcess::NotRunning) {
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

QString MainWindow::formatBridgeStatus(const QJsonObject &payload, const QString &area) const
{
    const QJsonObject project = payload.value(QStringLiteral("project")).toObject();
    const QJsonObject workspace = payload.value(QStringLiteral("workspace")).toObject();
    const QJsonObject resources = payload.value(QStringLiteral("resources")).toObject();
    const QJsonObject memory = resources.value(QStringLiteral("memory")).toObject();
    const QJsonObject swap = resources.value(QStringLiteral("swap")).toObject();
    QStringList lines;
    lines << QStringLiteral("%1 · %2")
        .arg(project.value(QStringLiteral("name")).toString(), project.value(QStringLiteral("profile")).toString());

    if (area == QStringLiteral("skills")) {
        const QJsonArray skills = payload.value(QStringLiteral("skills")).toArray();
        lines << QStringLiteral("Skills habilitadas: %1 · catálogo: %2")
            .arg(QString::number(payload.value(QStringLiteral("enabled_skill_count")).toInt()),
                 QString::number(skills.size()));
        for (const auto &value : skills) {
            const auto skill = value.toObject();
            QStringList details;
            if (skill.value(QStringLiteral("enabled")).toBool()) {
                details << QStringLiteral("habilitada");
            }
            for (const auto &reason : skill.value(QStringLiteral("reasons")).toArray()) {
                details << reason.toString();
            }
            lines << QStringLiteral("%1 — %2%3")
                .arg(skill.value(QStringLiteral("name")).toString(),
                     skill.value(QStringLiteral("description")).toString(),
                     details.isEmpty() ? QString() : QStringLiteral(" · ") + details.join(QStringLiteral("; ")));
        }
        return lines.join(QLatin1Char('\n'));
    }
    if (area == QStringLiteral("packs")) {
        const QJsonArray packs = payload.value(QStringLiteral("packs")).toArray();
        lines << QStringLiteral("Tech Packs: %1").arg(packs.size());
        for (const auto &value : packs) {
            const auto pack = value.toObject();
            QStringList skillNames;
            for (const auto &skill : pack.value(QStringLiteral("skills")).toArray()) {
                skillNames << skill.toString();
            }
            QString detail = QStringLiteral("%1 — %2\n  Skills: %3")
                .arg(pack.value(QStringLiteral("name")).toString())
                .arg(pack.value(QStringLiteral("description")).toString())
                .arg(skillNames.join(QStringLiteral(", ")));
            QStringList reasons;
            for (const auto &reason : pack.value(QStringLiteral("reasons")).toArray()) {
                reasons << reason.toString();
            }
            if (!reasons.isEmpty()) {
                detail += QStringLiteral("\n  Detectado: ") + reasons.join(QStringLiteral("; "));
            }
            lines << detail;
        }
        return lines.join(QLatin1Char('\n'));
    }

    if (area == QStringLiteral("projects")) {
        const QJsonArray registered = payload.value(QStringLiteral("registered_projects")).toArray();
        lines << QStringLiteral("Proyectos registrados: %1").arg(registered.size());
        for (const auto &value : registered) {
            const auto item = value.toObject();
            const QString trust = item.value(QStringLiteral("trusted")).toBool()
                ? QStringLiteral("confiable") : QStringLiteral("requiere trust");
            lines << QStringLiteral("%1 · %2 · %3 · %4 · %5")
                .arg(item.value(QStringLiteral("alias")).toString(),
                     item.value(QStringLiteral("name")).toString(),
                     item.value(QStringLiteral("profile")).toString(),
                     item.value(QStringLiteral("status")).toString(), trust);
        }
        return lines.join(QLatin1Char('\n'));
    }
    if (area == QStringLiteral("project")) {
        lines << QStringLiteral("Confianza: %1 · estado: %2")
            .arg(workspace.value(QStringLiteral("trusted")).toBool() ? QStringLiteral("confiable") : QStringLiteral("requiere trust"),
                 workspace.value(QStringLiteral("status")).toString());
        lines << QStringLiteral("Sesión: %1")
            .arg(workspace.value(QStringLiteral("session_name")).toString(QStringLiteral("sin sesión")));
        lines << QStringLiteral("Procesos configurados: %1 · agentes configurados: %2")
            .arg(QString::number(workspace.value(QStringLiteral("configured_process_count")).toInt()),
                 QString::number(workspace.value(QStringLiteral("configured_agent_count")).toInt()));
        for (const auto &warning : workspace.value(QStringLiteral("warnings")).toArray()) {
            lines << QStringLiteral("Aviso: %1").arg(warning.toString());
        }
        if (!workspace.value(QStringLiteral("error")).isNull()) {
            lines << QStringLiteral("Configuración: %1").arg(workspace.value(QStringLiteral("error")).toString());
        }
        return lines.join(QLatin1Char('\n'));
    }
    if (area == QStringLiteral("agents")) {
        lines << QStringLiteral("Agentes configurados: %1").arg(workspace.value(QStringLiteral("configured_agent_count")).toInt());
        for (const auto &value : payload.value(QStringLiteral("agents")).toArray()) {
            const auto agent = value.toObject();
            lines << QStringLiteral("%1 · %2 · sesión %3 · pane %4")
                .arg(agent.value(QStringLiteral("id")).toString(),
                     agent.value(QStringLiteral("available")).toBool() ? QStringLiteral("disponible") : QStringLiteral("no disponible"),
                     agent.value(QStringLiteral("session")).toString(),
                     agent.value(QStringLiteral("pane")).toString());
        }
        return lines.join(QLatin1Char('\n'));
    }
    if (area == QStringLiteral("processes")) {
        lines << QStringLiteral("Procesos configurados: %1").arg(workspace.value(QStringLiteral("configured_process_count")).toInt());
        for (const auto &value : payload.value(QStringLiteral("processes")).toArray()) {
            const auto process = value.toObject();
            QString detail = QStringLiteral("%1 · %2")
                .arg(process.value(QStringLiteral("label")).toString(), process.value(QStringLiteral("status")).toString());
            if (process.value(QStringLiteral("pid")).isDouble()) {
                detail += QStringLiteral(" · PID %1").arg(process.value(QStringLiteral("pid")).toInt());
            }
            if (process.value(QStringLiteral("port")).isDouble()) {
                detail += QStringLiteral(" · puerto %1").arg(process.value(QStringLiteral("port")).toInt());
            }
            lines << detail;
        }
        return lines.join(QLatin1Char('\n'));
    }
    if (area == QStringLiteral("resources")) {
        const QJsonObject processResources = resources.value(QStringLiteral("processes")).toObject();
        const QJsonObject governor = resources.value(QStringLiteral("governor")).toObject();
        lines << QStringLiteral("RAM: %1 / %2 · %3")
            .arg(formatBytes(memory.value(QStringLiteral("used_bytes")).toDouble()),
                 formatBytes(memory.value(QStringLiteral("total_bytes")).toDouble()),
                 displayPercent(memory.value(QStringLiteral("percent"))));
        lines << QStringLiteral("Swap: %1 / %2 · %3")
            .arg(formatBytes(swap.value(QStringLiteral("used_bytes")).toDouble()),
                 formatBytes(swap.value(QStringLiteral("total_bytes")).toDouble()),
                 displayPercent(swap.value(QStringLiteral("percent"))));
        lines << QStringLiteral("CPU: %1 · procesos del proyecto: %2, %3")
            .arg(displayPercent(resources.value(QStringLiteral("cpu_percent"))),
                 QString::number(processResources.value(QStringLiteral("running_count")).toInt()),
                 formatBytes(processResources.value(QStringLiteral("rss_bytes")).toDouble()));
        lines << QStringLiteral("Governor: aviso %1% · crítico %2% · swap %3% · máximo %4 agentes")
            .arg(QString::number(governor.value(QStringLiteral("warning_memory_percent")).toInt()),
                 QString::number(governor.value(QStringLiteral("critical_memory_percent")).toInt()),
                 QString::number(governor.value(QStringLiteral("warning_swap_percent")).toInt()),
                 QString::number(governor.value(QStringLiteral("max_agents")).toInt()));
        return lines.join(QLatin1Char('\n'));
    }

    const QJsonArray stacks = project.value(QStringLiteral("stacks")).toArray();
    QStringList stackNames;
    for (const auto &stack : stacks) {
        stackNames << stack.toString();
    }
    lines << QStringLiteral("Stack: %1").arg(stackNames.isEmpty() ? QStringLiteral("sin detectar") : stackNames.join(QStringLiteral(", ")));
    lines << QStringLiteral("Confianza: %1 · Workspace: %2")
        .arg(workspace.value(QStringLiteral("trusted")).toBool() ? QStringLiteral("confiable") : QStringLiteral("requiere trust"),
             workspace.value(QStringLiteral("status")).toString());
    lines << QStringLiteral("RAM: %1 · Swap: %2 · CPU: %3")
        .arg(displayPercent(memory.value(QStringLiteral("percent"))),
             displayPercent(swap.value(QStringLiteral("percent"))),
             displayPercent(resources.value(QStringLiteral("cpu_percent"))));
    return lines.join(QLatin1Char('\n'));
}

QString MainWindow::formatResourcesOverview(const QJsonObject &payload) const
{
    const QJsonObject system = payload.value(QStringLiteral("system")).toObject();
    QStringList lines;
    lines << QStringLiteral("Sistema · RAM %1 / %2 (%3) · swap %4 / %5 (%6) · CPU %7")
        .arg(formatBytes(system.value(QStringLiteral("memory_used_bytes")).toDouble()),
             formatBytes(system.value(QStringLiteral("memory_total_bytes")).toDouble()),
             displayPercent(system.value(QStringLiteral("memory_percent"))),
             formatBytes(system.value(QStringLiteral("swap_used_bytes")).toDouble()),
             formatBytes(system.value(QStringLiteral("swap_total_bytes")).toDouble()),
             displayPercent(system.value(QStringLiteral("swap_percent"))),
             displayPercent(system.value(QStringLiteral("cpu_percent"))));
    lines << QStringLiteral("Consumo de procesos gestionados por proyecto:");
    for (const auto &value : payload.value(QStringLiteral("projects")).toArray()) {
        const auto project = value.toObject();
        const QString trust = project.value(QStringLiteral("trusted")).toBool()
            ? QStringLiteral("confiable") : QStringLiteral("sin trust");
        lines << QStringLiteral("%1 · %2 · %3 · %4 procesos · RAM %5 · CPU %6 · Governor %7/%8 · agentes %9 · %10")
            .arg(project.value(QStringLiteral("alias")).toString(),
                 project.value(QStringLiteral("name")).toString(),
                 project.value(QStringLiteral("status")).toString(),
                 QString::number(project.value(QStringLiteral("running_count")).toInt()),
                 formatBytes(project.value(QStringLiteral("rss_bytes")).toDouble()),
                 displayPercent(project.value(QStringLiteral("cpu_percent"))),
                 QString::number(project.value(QStringLiteral("warning_memory_percent")).toInt()),
                 QString::number(project.value(QStringLiteral("critical_memory_percent")).toInt()),
                 QString::number(project.value(QStringLiteral("max_agents")).toInt()), trust);
    }
    if (payload.value(QStringLiteral("projects")).toArray().isEmpty()) {
        lines << QStringLiteral("No hay proyectos registrados.");
    }
    return lines.join(QLatin1Char('\n'));
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
