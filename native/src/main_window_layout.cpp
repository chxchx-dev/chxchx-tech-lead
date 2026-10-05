#include "main_window.hpp"
#include "main_window_areas.hpp"
#include "integrations/bridge_client.hpp"
#include "integrations/agent_session_widget.hpp"

#include <QComboBox>
#include <QAction>
#include <QDir>
#include <QDockWidget>
#include <QFileSystemModel>
#include <QFormLayout>
#include <QFont>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QListWidgetItem>
#include <QMenu>
#include <QMenuBar>
#include <QPlainTextEdit>
#include <QProcess>
#include <QPushButton>
#include <QSplitter>
#include <QStackedWidget>
#include <QStatusBar>
#include <QTabWidget>
#include <QTreeView>
#include <QToolBar>
#include <QVBoxLayout>

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
    projectDock->setFeatures(QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetFloatable
        | QDockWidget::DockWidgetClosable);
    projectDock->setWidget(m_projectTree);
    addDockWidget(Qt::LeftDockWidgetArea, projectDock);

    m_editorTabs = new QTabWidget(this);
    m_editorTabs->setTabsClosable(true);
    m_editorTabs->setMovable(true);
    connect(m_editorTabs, &QTabWidget::tabCloseRequested, this, &MainWindow::closeEditorTab);
    connect(m_editorTabs, &QTabWidget::currentChanged, this, [this](int index) {
        auto *session = qobject_cast<AgentSessionWidget *>(m_editorTabs->widget(index));
        if (session != nullptr) session->focusTerminal();
    });
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
                       "• Actividad conserva la salida reciente de consultas y operaciones; ábrela desde Ver o desde su botón en la barra.\n"
                       "• La paleta puede abrir una terminal Zellij nueva o adjuntarse al workspace en un emulador externo.\n"
                       "• Las vistas de Memoria, Chats y Errores son de solo lectura.\n"
                       "• Agentes abre la interfaz interactiva configurada dentro de una pestaña junto a los archivos.\n"
                       "• Configuración integra MCP en todos los clientes o solo Claude Code, Codex u OpenCode.\n"
                       "• Skills instala o quita skills del proyecto; después sincroniza instrucciones. Packs aplica grupos detectados.\n"
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
    controlDock->setFeatures(QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetFloatable
        | QDockWidget::DockWidgetClosable);
    controlDock->setWidget(control);
    addDockWidget(Qt::RightDockWidgetArea, controlDock);

    m_output = new QPlainTextEdit(this);
    m_output->setReadOnly(true);
    m_output->setMaximumBlockCount(4000);
    m_activityDock = new QDockWidget(QStringLiteral("Actividad"), this);
    m_activityDock->setObjectName(QStringLiteral("activityDock"));
    m_activityDock->setFeatures(QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetFloatable
        | QDockWidget::DockWidgetClosable);
    auto *activityContent = new QWidget(m_activityDock);
    auto *activityLayout = new QVBoxLayout(activityContent);
    activityLayout->setContentsMargins(6, 6, 6, 6);
    auto *activityToolbar = new QHBoxLayout();
    activityToolbar->addWidget(new QLabel(QStringLiteral("Registro reciente de operaciones"), activityContent), 1);
    auto *clearActivity = new QPushButton(QStringLiteral("Limpiar"), activityContent);
    activityToolbar->addWidget(clearActivity);
    activityLayout->addLayout(activityToolbar);
    activityLayout->addWidget(m_output, 1);
    m_activityDock->setWidget(activityContent);
    addDockWidget(Qt::BottomDockWidgetArea, m_activityDock);
    m_activityDock->hide();

    auto *viewMenu = menuBar()->addMenu(QStringLiteral("Ver"));
    viewMenu->addAction(projectDock->toggleViewAction());
    viewMenu->addAction(m_activityDock->toggleViewAction());
    viewMenu->addAction(controlDock->toggleViewAction());
    auto *showActivity = new QAction(QStringLiteral("Actividad"), this);
    showActivity->setCheckable(true);
    showActivity->setToolTip(QStringLiteral("Mostrar u ocultar el registro de actividad"));
    // The activity toggle stays visible beside the main window's file actions.
    const auto toolbars = findChildren<QToolBar *>();
    if (!toolbars.isEmpty()) toolbars.first()->addAction(showActivity);
    connect(showActivity, &QAction::toggled, m_activityDock, &QDockWidget::setVisible);
    connect(m_activityDock, &QDockWidget::visibilityChanged, showActivity, &QAction::setChecked);
    connect(clearActivity, &QPushButton::clicked, m_output, &QPlainTextEdit::clear);

    m_bridgeClient = new BridgeClient(m_projectPath, this);
    connect(m_bridgeClient, &BridgeClient::finished, this, &MainWindow::finishCommand);
    connect(m_bridgeClient, &BridgeClient::failedToStart, this, &MainWindow::commandFailedToStart);
    selectArea(0);
}
