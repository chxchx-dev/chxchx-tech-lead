#pragma once

#include <QMainWindow>
#include <QProcess>
#include <QString>
#include <QStringList>
#include <QHash>

class QJsonObject;
class BridgeClient;
class AgentSessionWidget;
class QDockWidget;
class QDialog;
class QComboBox;
class QFileSystemModel;
class QLabel;
class QLineEdit;
class QListWidget;
class QListWidgetItem;
class QModelIndex;
class QPlainTextEdit;
class ScintillaEditBase;
class QPushButton;
class QStackedWidget;
class QTabWidget;
class QTreeView;
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
    void closeEditorTab(int index);
    void openTreeFile(const QModelIndex &index);
    void selectArea(int row);
    void refreshArea();
    void finishCommand(int exitCode, QProcess::ExitStatus status);
    void commandFailedToStart(const QString &reason);
    void performPrimaryAreaAction();
    void performSecondaryAreaAction();
    void performTertiaryAreaAction();
    void performQuaternaryAreaAction();
    void selectMemoryNote(QListWidgetItem *item);
    void selectChatConversation(QListWidgetItem *item);
    void openCommandPalette();
    void executePaletteCommand(QListWidgetItem *item);
    void openNewWorkspaceTerminal();
    void attachWorkspaceTerminal();
    void attachAgentTerminal();
    void openSelectedAgent(bool newChat = false);
    void openAllAgentSessions();

private:
    void buildActions();
    void buildLayout();
    void openPath(const QString &path);
    ScintillaEditBase *currentEditor() const;
    QString currentFilePath() const;
    QStringList readCommandForArea(const QString &area) const;
    QString formatBridgeStatus(const QJsonObject &payload, const QString &area) const;
    QString formatResourcesOverview(const QJsonObject &payload) const;
    void configureAreaActions();
    void updateAreaActionState();
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
    void schedulePendingRefresh();
    void appendOutput(const QString &text);

    QString m_projectPath;
    QString m_currentArea = QStringLiteral("overview");
    QString m_commandArea;
    QFileSystemModel *m_fileModel = nullptr;
    QTreeView *m_projectTree = nullptr;
    QListWidget *m_areaList = nullptr;
    QComboBox *m_targetSelector = nullptr;
    QPushButton *m_primaryAction = nullptr;
    QPushButton *m_secondaryAction = nullptr;
    QPushButton *m_tertiaryAction = nullptr;
    QPushButton *m_quaternaryAction = nullptr;
    QTabWidget *m_editorTabs = nullptr;
    QStackedWidget *m_mainPages = nullptr;
    QWidget *m_handoffPage = nullptr;
    QWidget *m_memoryPage = nullptr;
    QWidget *m_chatsPage = nullptr;
    QWidget *m_errorsPage = nullptr;
    QWidget *m_setupPage = nullptr;
    QWidget *m_guidePage = nullptr;
    QWidget *m_brandPage = nullptr;
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
    QLabel *m_areaDescription = nullptr;
    BridgeClient *m_bridgeClient = nullptr;
    QStringList m_confirmedActionArguments;
    QHash<QString, AgentSessionWidget *> m_agentSessions;
    QStringList m_forceActionArguments;
    QString m_confirmedActionTitle;
    QString m_pendingProjectPath;
    QString m_workspaceStatus;
    bool m_projectTrusted = false;
    bool m_previewPending = false;
    bool m_governorRetryPending = false;
    bool m_refreshAfterAction = false;
    bool m_refreshQueued = false;
};
