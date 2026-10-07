#pragma once

#include <QMainWindow>
#include <QProcess>
#include <QString>
#include <QStringList>
#include <QHash>
#include <QPointer>

class QJsonObject;
class QAction;
class BridgeClient;
class ProjectFileIndex;
class AgentSessionWidget;
class WorkspaceTerminalWidget;
class QDockWidget;
class QCloseEvent;
class QDialog;
class QComboBox;
class QFileSystemModel;
class QLabel;
class QLineEdit;
class QListWidget;
class QListWidgetItem;
class QModelIndex;
class QMenu;
class QPlainTextEdit;
class QSortFilterProxyModel;
class ScintillaEditBase;
class QPushButton;
class QStackedWidget;
class QSplitter;
class QTabWidget;
class QPoint;
class QTreeView;
class QToolBar;
class QWidget;

class MainWindow final : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(QString projectPath, QWidget *parent = nullptr);

private slots:
    void openFile();
    void saveFile();
    void saveFileAs();
    void findInCurrentFile();

    void openTreeFile(const QModelIndex &index);
    void selectArea(int row);
    void refreshArea();
    void finishCommand(int exitCode, QProcess::ExitStatus status);
    void commandFailedToStart(const QString &reason);
    void performPrimaryAreaAction();
    void performSecondaryAreaAction();
    void performTertiaryAreaAction();
    void performQuaternaryAreaAction();
    void trustProjectFromHome();
    void startWorkspaceFromHome();
    void suspendWorkspaceFromHome();
    void stopWorkspaceFromHome();
    void prepareProjectFromHome();
    void syncSkillsFromHome();
    void doctorFromHome();
    void openHomeSelectedAgent(bool newChat = false);
    void openAllHomeAgentSessions();
    void selectMemoryNote(QListWidgetItem *item);
    void selectChatConversation(QListWidgetItem *item);
    void openCommandPalette();
    void searchProjectFiles();
    void executePaletteCommand(QListWidgetItem *item);
    void showAreaDashboard(const QJsonObject &payload, const QString &area);
    void updateAgentDashboard();
    void openNewWorkspaceTerminal();
    void attachWorkspaceTerminal();
    void attachWorkspaceTerminalExternal();
    void attachAgentTerminal();
    void openSelectedAgent(bool newChat = false);
    void openAllAgentSessions();

private:
    void closeEvent(QCloseEvent *event) override;
    void buildActions();
    void buildLayout();
    void configureShortcuts();
    void setWordWrapEnabled(bool enabled);
    void performEditAction(const QString &actionId);
    void updateEditActionState();
    QList<QTabWidget *> editorTabGroups() const;
    QTabWidget *activeEditorTabs() const;
    QTabWidget *tabGroupFor(QWidget *page) const;
    void closeEditorTab(QTabWidget *tabs, int index);
    void moveTabToOtherGroup(QTabWidget *source, int index, Qt::Orientation orientation);
    void closeSecondaryTabGroup();
    void showTabContextMenu(QTabWidget *tabs, const QPoint &position);
    void openPath(const QString &path, int lineNumber = 0);
    ScintillaEditBase *currentEditor() const;
    QString currentFilePath() const;
    QStringList readCommandForArea(const QString &area) const;
    QString formatBridgeStatus(const QJsonObject &payload, const QString &area) const;
    QString formatResourcesOverview(const QJsonObject &payload) const;
    void configureAreaActions();
    void updateAreaActionState();
    void updateHomeActionState();
    void launchAgentFromSelector(QComboBox *selector, bool newChat);
    void openAgentSessions(QComboBox *selector);
    void updateAreaTargets(const QJsonObject &payload, const QString &area);
    void showHandoff(const QJsonObject &payload);
    void showMemory(const QJsonObject &payload);
    void showConversations(const QJsonObject &payload);
    void showConversation(const QJsonObject &payload);
    void showErrors(const QJsonObject &payload);
    void setProjectRoot(const QString &path);
    void runCommand(const QStringList &arguments);
    bool runPreview(
        const QStringList &previewArguments,
        const QStringList &actionArguments,
        const QStringList &forceArguments,
        const QString &title);
    void createEmbeddedAgentSession(
        const QString &agentId,
        const QString &program,
        const QStringList &arguments,
        const QString &workingDirectory,
        bool newChat);
    void createEmbeddedWorkspaceTerminal();
    void createEmbeddedWorkspaceAttach(const QStringList &command);
    int currentProjectEmbeddedAgentCount() const;
    void schedulePendingRefresh();
    void appendOutput(const QString &text);
    QString projectSettingsGroup() const;
    bool isProjectFilePathSafe(const QString &path) const;
    void restoreEditorSession();
    void saveEditorSession() const;
    QStringList recentProjectFiles() const;
    void recordRecentProjectFile(const QString &path);

    QPointer<QWidget> m_editTargetWidget;
    QString m_projectPath;
    QString m_currentArea = QStringLiteral("overview");
    QString m_commandArea;
    QFileSystemModel *m_fileModel = nullptr;
    QSortFilterProxyModel *m_projectProxy = nullptr;
    QTreeView *m_projectTree = nullptr;
    QLineEdit *m_projectSearch = nullptr;
    QListWidget *m_areaList = nullptr;
    QComboBox *m_targetSelector = nullptr;
    QPushButton *m_primaryAction = nullptr;
    QPushButton *m_secondaryAction = nullptr;
    QPushButton *m_tertiaryAction = nullptr;
    QPushButton *m_quaternaryAction = nullptr;
    QTabWidget *m_editorTabs = nullptr;
    QTabWidget *m_secondaryEditorTabs = nullptr;
    QTabWidget *m_activeEditorTabs = nullptr;
    QSplitter *m_editorSplit = nullptr;
    QStackedWidget *m_mainPages = nullptr;
    QWidget *m_dashboardPage = nullptr;
    QLabel *m_dashboardTitle = nullptr;
    QLabel *m_dashboardSummary = nullptr;
    QListWidget *m_dashboardItems = nullptr;
    QWidget *m_homeQuickActions = nullptr;
    QComboBox *m_homeAgentSelector = nullptr;
    QPushButton *m_homeTrustButton = nullptr;
    QPushButton *m_homeWorkspaceButton = nullptr;
    QPushButton *m_homeSuspendButton = nullptr;
    QPushButton *m_homeStopButton = nullptr;
    QPushButton *m_homeTerminalButton = nullptr;
    QPushButton *m_homePrepareButton = nullptr;
    QPushButton *m_homeSyncSkillsButton = nullptr;
    QPushButton *m_homeDoctorButton = nullptr;
    QPushButton *m_homeAgentButton = nullptr;
    QPushButton *m_homeNewChatButton = nullptr;
    QPushButton *m_homeAllAgentsButton = nullptr;
    QWidget *m_handoffPage = nullptr;
    QWidget *m_memoryPage = nullptr;
    QWidget *m_chatsPage = nullptr;
    QWidget *m_errorsPage = nullptr;
    QWidget *m_setupPage = nullptr;
    QWidget *m_guidePage = nullptr;
    QWidget *m_brandPage = nullptr;
    QPushButton *m_guideSetupButton = nullptr;
    QPushButton *m_guideSkillsButton = nullptr;
    QPushButton *m_guideProjectButton = nullptr;
    QPushButton *m_guideAgentsButton = nullptr;
    QLineEdit *m_handoffSummary = nullptr;
    QLineEdit *m_handoffPending = nullptr;
    QLineEdit *m_handoffValidation = nullptr;
    QPlainTextEdit *m_handoffPreview = nullptr;
    QLineEdit *m_memorySearch = nullptr;
    QListWidget *m_memoryList = nullptr;
    QLabel *m_memorySummary = nullptr;
    QPlainTextEdit *m_memoryDetail = nullptr;
    QLineEdit *m_chatSearch = nullptr;
    QListWidget *m_chatList = nullptr;
    QLabel *m_chatSummary = nullptr;
    QPlainTextEdit *m_chatDetail = nullptr;
    QLabel *m_errorsSummary = nullptr;
    QListWidget *m_errorsList = nullptr;
    QPlainTextEdit *m_errorDetail = nullptr;
    QPlainTextEdit *m_output = nullptr;
    QDockWidget *m_activityDock = nullptr;
    QDockWidget *m_projectDock = nullptr;
    QDockWidget *m_controlDock = nullptr;
    QMenu *m_viewMenu = nullptr;
    QToolBar *m_quickToolbar = nullptr;
    QLabel *m_areaDescription = nullptr;
    BridgeClient *m_bridgeClient = nullptr;
    ProjectFileIndex *m_fileIndex = nullptr;
    QStringList m_confirmedActionArguments;
    QHash<QString, AgentSessionWidget *> m_agentSessions;
    QHash<QString, QString> m_agentSessionProjects;
    QHash<QString, QAction *> m_shortcutActions;
    QStringList m_shortcutOrder;
    QStringList m_forceActionArguments;
    QString m_confirmedActionTitle;
    QString m_pendingProjectPath;
    QString m_workspaceStatus;
    int m_nextTerminalNumber = 1;
    bool m_wordWrapEnabled = true;
    bool m_projectTrusted = false;
    bool m_previewPending = false;
    bool m_governorRetryPending = false;
    bool m_refreshAfterAction = false;
    bool m_refreshQueued = false;
    bool m_restoringEditorSession = false;
};
