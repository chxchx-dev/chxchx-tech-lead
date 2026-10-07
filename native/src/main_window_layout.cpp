#include "main_window.hpp"
#include "main_window_areas.hpp"
#include "integrations/bridge_client.hpp"
#include "integrations/agent_session_widget.hpp"
#include "studio_icons.hpp"

#include <ScintillaEditBase.h>

#include <QComboBox>
#include <QAction>
#include <QDir>
#include <QDockWidget>
#include <QFileSystemModel>
#include <QFrame>
#include <QFormLayout>
#include <QFont>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QListWidgetItem>
#include <QPixmap>
#include <QMenu>
#include <QMenuBar>
#include <QPlainTextEdit>
#include <QProcess>
#include <QPushButton>
#include <QSortFilterProxyModel>
#include <QSplitter>
#include <QStackedWidget>
#include <QStatusBar>
#include <QTabBar>
#include <QTabWidget>
#include <QTreeView>
#include <QToolBar>
#include <QVBoxLayout>

#include <utility>

namespace {

QIcon areaIcon(const QString &id)
{
    struct IconEntry {
        const char *id;
        const char *themeName;
        QStyle::StandardPixmap fallback;
    };
    static constexpr IconEntry icons[] = {
        {"overview", "go-home", QStyle::SP_DirHomeIcon},
        {"project", "folder-open", QStyle::SP_DirOpenIcon},
        {"agents", "system-run", QStyle::SP_CommandLink},
        {"processes", "utilities-system-monitor", QStyle::SP_MediaPlay},
        {"projects", "folder", QStyle::SP_DirIcon},
        {"skills", "preferences-plugin", QStyle::SP_DialogApplyButton},
        {"packs", "package-x-generic", QStyle::SP_DirLinkIcon},
        {"resources", "drive-harddisk", QStyle::SP_DriveHDIcon},
        {"handoff", "document-edit", QStyle::SP_FileIcon},
        {"memory", "view-history", QStyle::SP_FileDialogInfoView},
        {"chats", "chat-message", QStyle::SP_MessageBoxInformation},
        {"errors", "dialog-warning", QStyle::SP_MessageBoxWarning},
        {"setup", "preferences-system", QStyle::SP_ComputerIcon},
        {"guide", "help-contents", QStyle::SP_DialogHelpButton},
        {"brand", "applications-development", QStyle::SP_TitleBarMenuButton},
    };
    for (const auto &entry : icons) {
        if (id == QString::fromUtf8(entry.id)) {
            return studioIcon(QString::fromUtf8(entry.themeName), entry.fallback);
        }
    }
    return studioIcon(QStringLiteral("applications-development"), QStyle::SP_FileIcon);
}

class ProjectTreeProxyModel final : public QSortFilterProxyModel {
public:
    ProjectTreeProxyModel(QString rootPath, QFileSystemModel *source, QObject *parent)
        : QSortFilterProxyModel(parent), m_rootPath(QDir(std::move(rootPath)).absolutePath())
    {
        setSourceModel(source);
        setRecursiveFilteringEnabled(true);
        setFilterCaseSensitivity(Qt::CaseInsensitive);
        setFilterKeyColumn(0);
    }

protected:
    bool filterAcceptsRow(int sourceRow, const QModelIndex &sourceParent) const override
    {
        const QModelIndex sourceIndex = sourceModel()->index(sourceRow, 0, sourceParent);
        if (sourceIndex.data(QFileSystemModel::FilePathRole).toString() == m_rootPath) return true;
        return QSortFilterProxyModel::filterAcceptsRow(sourceRow, sourceParent);
    }

private:
    QString m_rootPath;
};

} // namespace

void MainWindow::buildLayout()
{
    m_fileModel = new QFileSystemModel(this);
    m_fileModel->setFilter(QDir::AllEntries | QDir::NoDotAndDotDot | QDir::Hidden | QDir::System);
    m_fileModel->setRootPath(m_projectPath);
    m_projectProxy = new ProjectTreeProxyModel(m_projectPath, m_fileModel, this);

    m_projectTree = new QTreeView(this);
    m_projectTree->setModel(m_projectProxy);
    m_projectTree->setRootIndex(m_projectProxy->mapFromSource(m_fileModel->index(m_projectPath)));
    m_projectTree->setHeaderHidden(true);
    m_projectTree->setIconSize(QSize(18, 18));
    m_projectTree->setToolTip(QStringLiteral("Un clic abre archivos o despliega/contrae carpetas."));
    m_projectTree->setAnimated(true);
    m_projectTree->setIndentation(16);
    m_projectTree->setUniformRowHeights(true);
    m_projectTree->setExpandsOnDoubleClick(false);
    for (int column = 1; column < m_fileModel->columnCount(); ++column) {
        m_projectTree->hideColumn(column);
    }
    connect(m_projectTree, &QTreeView::clicked, this, &MainWindow::openTreeFile);

    m_projectSearch = new QLineEdit(this);
    m_projectSearch->setPlaceholderText(QStringLiteral("Filtrar nombres en el árbol…"));
    m_projectSearch->setClearButtonEnabled(true);
    auto *projectContent = new QWidget(this);
    auto *projectLayout = new QVBoxLayout(projectContent);
    projectLayout->setContentsMargins(5, 5, 5, 5);
    projectLayout->setSpacing(5);
    projectLayout->addWidget(m_projectSearch);
    projectLayout->addWidget(m_projectTree, 1);

    m_projectDock = new QDockWidget(QStringLiteral("Explorador"), this);
    m_projectDock->setObjectName(QStringLiteral("projectDock"));
    m_projectDock->setFeatures(QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetFloatable
        | QDockWidget::DockWidgetClosable);
    m_projectDock->setWidget(projectContent);
    addDockWidget(Qt::LeftDockWidgetArea, m_projectDock);
    connect(m_projectSearch, &QLineEdit::textChanged, this,
        [this](const QString &text) { m_projectProxy->setFilterFixedString(text.trimmed()); });

    const auto configureEditorTabs = [this](QTabWidget *tabs) {
        tabs->setDocumentMode(true);
        tabs->setElideMode(Qt::ElideMiddle);
        tabs->setUsesScrollButtons(true);
        tabs->setIconSize(QSize(14, 14));
        tabs->setTabsClosable(true);
        tabs->setMovable(true);
        tabs->tabBar()->setContextMenuPolicy(Qt::CustomContextMenu);
        connect(tabs, &QTabWidget::tabCloseRequested, this,
            [this, tabs](int index) { closeEditorTab(tabs, index); });
        connect(tabs, &QTabWidget::currentChanged, this, [this, tabs](int index) {
            m_activeEditorTabs = tabs;
            auto *session = qobject_cast<AgentSessionWidget *>(tabs->widget(index));
            if (session != nullptr) {
                session->focusTerminal();
                return;
            }
            auto *editor = qobject_cast<ScintillaEditBase *>(tabs->widget(index));
            if (editor != nullptr) {
                statusBar()->showMessage(QDir(m_projectPath).relativeFilePath(
                    editor->property("filePath").toString()));
                editor->setFocus(Qt::OtherFocusReason);
            }
        });
        connect(tabs->tabBar(), &QTabBar::tabBarClicked, this,
            [this, tabs](int) { m_activeEditorTabs = tabs; });
        connect(tabs->tabBar(), &QTabBar::customContextMenuRequested, this,
            [this, tabs](const QPoint &position) { showTabContextMenu(tabs, position); });
    };
    m_editorTabs = new QTabWidget(this);
    configureEditorTabs(m_editorTabs);
    m_activeEditorTabs = m_editorTabs;
    m_secondaryEditorTabs = new QTabWidget(this);
    configureEditorTabs(m_secondaryEditorTabs);
    m_secondaryEditorTabs->hide();
    m_editorSplit = new QSplitter(Qt::Horizontal, this);
    m_editorSplit->setChildrenCollapsible(false);
    m_editorSplit->addWidget(m_editorTabs);
    m_editorSplit->addWidget(m_secondaryEditorTabs);
    m_editorSplit->setStretchFactor(0, 1);
    m_editorSplit->setStretchFactor(1, 1);
    m_mainPages = new QStackedWidget(this);
    const int sectionTab = m_editorTabs->addTab(m_mainPages, areaIcon(QStringLiteral("overview")),
        QString());
    m_editorTabs->setTabToolTip(sectionTab, QStringLiteral("Inicio · Secciones del proyecto"));
    m_editorTabs->tabBar()->setTabButton(sectionTab, QTabBar::RightSide, nullptr);
    connect(m_editorTabs->tabBar(), &QTabBar::tabMoved, this, [this](int, int) {
        const int index = m_editorTabs->indexOf(m_mainPages);
        if (index >= 0) {
            m_editorTabs->tabBar()->setTabButton(index, QTabBar::RightSide, nullptr);
        }
    });

    m_dashboardPage = new QWidget(m_mainPages);
    auto *dashboardLayout = new QVBoxLayout(m_dashboardPage);
    dashboardLayout->setContentsMargins(28, 24, 28, 24);
    dashboardLayout->setSpacing(12);
    m_dashboardTitle = new QLabel(m_dashboardPage);
    QFont dashboardTitleFont = m_dashboardTitle->font();
    dashboardTitleFont.setPointSize(dashboardTitleFont.pointSize() + 7);
    dashboardTitleFont.setBold(true);
    m_dashboardTitle->setFont(dashboardTitleFont);
    m_areaDescription = new QLabel(m_dashboardPage);
    m_areaDescription->setWordWrap(true);
    m_areaDescription->setStyleSheet(QStringLiteral("color: #91a7c2; font-size: 14px;"));
    m_dashboardSummary = new QLabel(m_dashboardPage);
    m_dashboardSummary->setWordWrap(true);
    m_dashboardSummary->setStyleSheet(QStringLiteral(
        "background: #101c2e; border: 1px solid #24364f; border-left: 3px solid #20c5d4; "
        "border-radius: 8px; padding: 12px 14px; color: #dce8f7; font-weight: 600;"));
    auto *dashboardListTitle = new QLabel(QStringLiteral("Estado actual"), m_dashboardPage);
    dashboardListTitle->setStyleSheet(QStringLiteral("color: #78a9ff; font-weight: 700; margin-top: 5px;"));
    m_dashboardItems = new QListWidget(m_dashboardPage);
    m_dashboardItems->setIconSize(QSize(20, 20));
    m_dashboardItems->setSpacing(3);
    dashboardLayout->addWidget(m_dashboardTitle);
    dashboardLayout->addWidget(m_areaDescription);
    dashboardLayout->addWidget(m_dashboardSummary);
    dashboardLayout->addWidget(dashboardListTitle);
    dashboardLayout->addWidget(m_dashboardItems, 1);
    m_mainPages->addWidget(m_dashboardPage);

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
    guideLayout->setContentsMargins(28, 24, 28, 24);
    guideLayout->setSpacing(12);
    auto *guideTitle = new QLabel(QStringLiteral("Guía rápida de ChxChx Studio"), m_guidePage);
    QFont guideTitleFont = guideTitle->font();
    guideTitleFont.setPointSize(guideTitleFont.pointSize() + 4);
    guideTitleFont.setBold(true);
    guideTitle->setFont(guideTitleFont);
    guideLayout->addWidget(guideTitle);
    auto *guideIntro = new QLabel(QStringLiteral(
        "Sigue esta ruta para preparar el proyecto, elegir contexto y comenzar una sesión con un agente."),
        m_guidePage);
    guideIntro->setWordWrap(true);
    guideIntro->setStyleSheet(QStringLiteral("color: #91a7c2; font-size: 14px;"));
    guideLayout->addWidget(guideIntro);

    const auto addGuideStep = [this, guideLayout](const QString &number, const QString &title,
        const QString &description, const QString &buttonText) {
        auto *card = new QFrame(m_guidePage);
        card->setObjectName(QStringLiteral("guideStepCard"));
        card->setStyleSheet(QStringLiteral(
            "QFrame#guideStepCard { background: #101c2e; border: 1px solid #263a55; "
            "border-radius: 8px; }"));
        auto *row = new QHBoxLayout(card);
        row->setContentsMargins(14, 11, 14, 11);
        row->setSpacing(13);
        auto *numberLabel = new QLabel(number, card);
        numberLabel->setStyleSheet(QStringLiteral("color: #63e6ee; font-size: 18px; font-weight: 700;"));
        numberLabel->setFixedWidth(30);
        row->addWidget(numberLabel, 0, Qt::AlignTop);
        auto *copy = new QVBoxLayout();
        copy->setSpacing(4);
        auto *stepTitle = new QLabel(title, card);
        stepTitle->setStyleSheet(QStringLiteral("font-weight: 700; color: #dce8f7;"));
        auto *stepDescription = new QLabel(description, card);
        stepDescription->setWordWrap(true);
        stepDescription->setStyleSheet(QStringLiteral("color: #91a7c2;"));
        copy->addWidget(stepTitle);
        copy->addWidget(stepDescription);
        row->addLayout(copy, 1);
        auto *button = new QPushButton(buttonText, card);
        row->addWidget(button, 0, Qt::AlignVCenter);
        guideLayout->addWidget(card);
        return button;
    };
    m_guideSetupButton = addGuideStep(QStringLiteral("01"), QStringLiteral("Prepara el proyecto"),
        QStringLiteral("En Configuración, elige Init completo y revisa la previsualización. Esto crea "
                       "las instrucciones y archivos .ai que usarán las sesiones."),
        QStringLiteral("Ir a Configuración"));
    m_guideSkillsButton = addGuideStep(QStringLiteral("02"), QStringLiteral("Elige las skills"),
        QStringLiteral("Revisa las recomendaciones para el stack, habilita las que quieras y sincroniza "
                       "las instrucciones del proyecto."), QStringLiteral("Ver Skills"));
    m_guideProjectButton = addGuideStep(QStringLiteral("03"), QStringLiteral("Autoriza el workspace"),
        QStringLiteral("En Proyecto, concede trust si corresponde. Inicia el workspace solo cuando "
                       "quieras levantar sus procesos; ChxChx volverá a revisar recursos."),
        QStringLiteral("Ir a Proyecto"));
    m_guideAgentsButton = addGuideStep(QStringLiteral("04"), QStringLiteral("Inicia con contexto"),
        QStringLiteral("En Agentes, elige Codex o Claude y usa Nueva sesión de agente. La sesión nueva "
                       "lee AGENTS.md y .ai; Basic Memory requiere estar configurado."),
        QStringLiteral("Ir a Agentes"));
    auto *dailyUse = new QLabel(QStringLiteral(
        "Después: abre archivos desde el explorador, usa Terminal + para una shell y haz clic derecho "
        "en una pestaña para dividirla. Los atajos se editan desde Edición → Configurar atajos."),
        m_guidePage);
    dailyUse->setWordWrap(true);
    dailyUse->setStyleSheet(QStringLiteral("color: #91a7c2; padding-top: 5px;"));
    guideLayout->addWidget(dailyUse);
    guideLayout->addStretch(1);
    m_mainPages->addWidget(m_guidePage);

    m_brandPage = new QWidget(m_mainPages);
    auto *brandLayout = new QVBoxLayout(m_brandPage);
    auto *brandLogo = new QLabel(m_brandPage);
    brandLogo->setAlignment(Qt::AlignHCenter | Qt::AlignVCenter);
    const QPixmap logo(QStringLiteral(":/brand/logo.png"));
    brandLogo->setPixmap(logo.scaled(280, 280, Qt::KeepAspectRatio, Qt::SmoothTransformation));
    brandLayout->addWidget(brandLogo);
    auto *brandSubtitle = new QLabel(QStringLiteral("TECH LEAD  ·  STUDIO"), m_brandPage);
    brandSubtitle->setStyleSheet(QStringLiteral("letter-spacing: 3px; color: #78a9ff;"));
    brandSubtitle->setAlignment(Qt::AlignHCenter);
    brandLayout->addWidget(brandSubtitle);
    auto *brandDescription = new QLabel(
        QStringLiteral("Control plane local para proyectos de desarrollo asistido.\n"
                       "Workspace, agentes, skills, memoria y recursos en una interfaz nativa."),
        m_brandPage);
    brandDescription->setWordWrap(true);
    brandDescription->setAlignment(Qt::AlignHCenter);
    brandLayout->addWidget(brandDescription);
    brandLayout->addStretch(1);
    m_mainPages->addWidget(m_brandPage);

    setCentralWidget(m_editorSplit);
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
    controlLayout->setContentsMargins(12, 12, 12, 12);
    controlLayout->setSpacing(9);
    auto *studioHeading = new QLabel(QStringLiteral("CHXCHX  /  STUDIO"), control);
    studioHeading->setStyleSheet(QStringLiteral("color: #63e6ee; font-weight: 700; letter-spacing: 2px;"));
    auto *projectHeading = new QLabel(QDir(m_projectPath).dirName(), control);
    projectHeading->setStyleSheet(QStringLiteral("color: #91a7c2; padding-bottom: 4px;"));
    projectHeading->setToolTip(m_projectPath);
    auto *areaSearch = new QLineEdit(control);
    areaSearch->setPlaceholderText(QStringLiteral("Filtrar secciones…"));
    areaSearch->setClearButtonEnabled(true);
    m_areaList = new QListWidget(control);
    m_areaList->setSpacing(2);
    m_areaList->setIconSize(QSize(18, 18));
    m_areaList->setMaximumWidth(300);
    for (const auto &area : areas) {
        auto *item = new QListWidgetItem(QString::fromUtf8(area.label), m_areaList);
        item->setData(Qt::UserRole, QString::fromUtf8(area.id));
        item->setIcon(areaIcon(QString::fromUtf8(area.id)));
    }
    m_areaList->setCurrentRow(0);
    controlLayout->addWidget(studioHeading);
    controlLayout->addWidget(projectHeading);
    controlLayout->addWidget(areaSearch);
    controlLayout->addWidget(m_areaList);
    connect(areaSearch, &QLineEdit::textChanged, this, [this](const QString &text) {
        int firstVisible = -1;
        for (int row = 0; row < m_areaList->count(); ++row) {
            auto *item = m_areaList->item(row);
            item->setHidden(!item->text().contains(text.trimmed(), Qt::CaseInsensitive));
            if (!item->isHidden() && firstVisible < 0) firstVisible = row;
        }
        if (firstVisible >= 0 && (m_areaList->currentItem() == nullptr
            || m_areaList->currentItem()->isHidden())) {
            m_areaList->setCurrentRow(firstVisible);
        }
        if (text.trimmed().isEmpty() && firstVisible >= 0 && m_areaList->currentRow() < 0) {
            m_areaList->setCurrentRow(firstVisible);
        }
    });
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
    const auto connectGuideNavigation = [this](QPushButton *button, const QString &areaId) {
        connect(button, &QPushButton::clicked, this, [this, areaId] {
            for (int row = 0; row < m_areaList->count(); ++row) {
                if (m_areaList->item(row)->data(Qt::UserRole).toString() == areaId) {
                    m_areaList->setCurrentRow(row);
                    return;
                }
            }
        });
    };
    connectGuideNavigation(m_guideSetupButton, QStringLiteral("setup"));
    connectGuideNavigation(m_guideSkillsButton, QStringLiteral("skills"));
    connectGuideNavigation(m_guideProjectButton, QStringLiteral("project"));
    connectGuideNavigation(m_guideAgentsButton, QStringLiteral("agents"));
    connect(m_targetSelector, &QComboBox::currentTextChanged, this, [this] { updateAreaActionState(); });
    connect(m_primaryAction, &QPushButton::clicked, this, &MainWindow::performPrimaryAreaAction);
    connect(m_secondaryAction, &QPushButton::clicked, this, &MainWindow::performSecondaryAreaAction);
    connect(m_tertiaryAction, &QPushButton::clicked, this, &MainWindow::performTertiaryAreaAction);
    connect(m_quaternaryAction, &QPushButton::clicked, this, &MainWindow::performQuaternaryAreaAction);

    m_controlDock = new QDockWidget(QStringLiteral("Secciones y acciones"), this);
    m_controlDock->setObjectName(QStringLiteral("controlDock"));
    m_controlDock->setFeatures(QDockWidget::DockWidgetMovable | QDockWidget::DockWidgetFloatable
        | QDockWidget::DockWidgetClosable);
    m_controlDock->setWidget(control);
    addDockWidget(Qt::RightDockWidgetArea, m_controlDock);

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

    auto *projectToggle = m_projectDock->toggleViewAction();
    auto *sectionsToggle = m_controlDock->toggleViewAction();
    auto *activityToggle = m_activityDock->toggleViewAction();
    projectToggle->setText(QStringLiteral("Explorador de archivos"));
    sectionsToggle->setText(QStringLiteral("Secciones y acciones"));
    projectToggle->setIcon(studioIcon(QStringLiteral("folder-open"), QStyle::SP_DirOpenIcon));
    sectionsToggle->setIcon(studioIcon(QStringLiteral("view-list-details"), QStyle::SP_FileDialogListView));
    activityToggle->setIcon(studioIcon(QStringLiteral("view-history"), QStyle::SP_FileDialogInfoView));
    projectToggle->setToolTip(QStringLiteral("Mostrar u ocultar el explorador de archivos"));
    sectionsToggle->setToolTip(QStringLiteral("Mostrar u ocultar las secciones y acciones"));
    activityToggle->setToolTip(QStringLiteral("Mostrar u ocultar la actividad"));
    m_viewMenu->addAction(projectToggle);
    m_viewMenu->addAction(activityToggle);
    m_viewMenu->addAction(sectionsToggle);
    m_quickToolbar->addSeparator();
    m_quickToolbar->addAction(projectToggle);
    m_quickToolbar->addAction(sectionsToggle);
    m_quickToolbar->addAction(activityToggle);
    connect(clearActivity, &QPushButton::clicked, m_output, &QPlainTextEdit::clear);

    m_bridgeClient = new BridgeClient(m_projectPath, this);
    connect(m_bridgeClient, &BridgeClient::finished, this, &MainWindow::finishCommand);
    connect(m_bridgeClient, &BridgeClient::failedToStart, this, &MainWindow::commandFailedToStart);
    selectArea(0);
}
