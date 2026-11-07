// The Git panel, its views and the diff view against a real temporary repository.
#include "../src/git/git_branches_view.hpp"
#include "../src/git/git_assistant_bar.hpp"
#include "../src/git/git_assistant_models.hpp"
#include "../src/git/git_changes_view.hpp"
#include "../src/git/git_diff_view.hpp"
#include "../src/git/git_explorer.hpp"
#include "../src/git/git_guide.hpp"
#include "../src/git/git_history_view.hpp"
#include "../src/git/git_commit_plan_view.hpp"
#include "../src/git/git_panel.hpp"
#include "../src/git/git_repository.hpp"

#include <QApplication>
#include <QDir>
#include <QElapsedTimer>
#include <QFile>
#include <QPointer>
#include <QProcess>
#include <QTemporaryDir>
#include <QTreeWidget>

#include <cstdio>
#include <functional>

namespace {

int fail(int code, const char *what)
{
    std::fprintf(stderr, "git ui smoke %d: %s\n", code, what);
    return code;
}

#define CHECK(code, condition) do { if (!(condition)) return fail(code, #condition); } while (0)

bool waitFor(const std::function<bool()> &condition, int timeoutMs = 15000)
{
    QElapsedTimer timer;
    timer.start();
    while (!condition() && timer.elapsed() < timeoutMs) QCoreApplication::processEvents(QEventLoop::AllEvents, 20);
    QCoreApplication::processEvents();
    return condition();
}

void write(const QString &path, const QByteArray &content)
{
    QDir().mkpath(QFileInfo(path).absolutePath());
    QFile file(path);
    if (file.open(QIODevice::WriteOnly)) file.write(content);
}

QByteArray read(const QString &path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

bool git(const QString &directory, const QStringList &args)
{
    QProcess process;
    process.setWorkingDirectory(directory);
    process.start("git", args);
    return process.waitForFinished(20000) && process.exitCode() == 0;
}

QByteArray gitOutput(const QString &directory, const QStringList &args)
{
    QProcess process;
    process.setWorkingDirectory(directory);
    process.start("git", args);
    process.waitForFinished(20000);
    return process.readAllStandardOutput();
}

} // namespace

int main(int argc, char **argv)
{
    QApplication application(argc, argv);
    QTemporaryDir home;
    qputenv("HOME", home.path().toUtf8());
    qputenv("GIT_CONFIG_NOSYSTEM", "1");
    qputenv("GIT_CONFIG_GLOBAL", home.filePath(".gitconfig").toUtf8());
    for (const char *name : {"GIT_AUTHOR_NAME", "GIT_COMMITTER_NAME"}) qputenv(name, "Ana");
    for (const char *name : {"GIT_AUTHOR_EMAIL", "GIT_COMMITTER_EMAIL"}) qputenv(name, "ana@example.com");

    // --- a folder that is not a repository -------------------------------------------------
    {
        QTemporaryDir plain;
        GitPanel panel(plain.path());
        CHECK(1, waitFor([&] { return !panel.stateText().isEmpty() && !panel.stateText().contains("Buscando"); }));
        CHECK(2, panel.stateText().contains("no usa control de versiones") && !panel.isRepository());
        bool initDone = false;
        panel.repository()->init([&initDone](const GitResult &r) { initDone = r.ok(); });
        CHECK(3, waitFor([&] { return initDone && panel.isRepository(); }));
        CHECK(4, panel.stateText().isEmpty());
    }

    QTemporaryDir dir;
    const QString root = dir.path();
    CHECK(10, git(root, {"init", "-q", "-b", "main"}));
    write(QDir(root).filePath("src/app.py"), "uno\ndos\ntres\n");
    write(QDir(root).filePath("README.md"), "# demo\n");
    CHECK(11, git(root, {"add", "-A"}) && git(root, {"commit", "-q", "-m", "Primer commit"}));
    CHECK(12, git(root, {"switch", "-q", "-c", "login"}));
    write(QDir(root).filePath("src/login.py"), "login\n");
    CHECK(13, git(root, {"add", "-A"}) && git(root, {"commit", "-q", "-m", "Añade el acceso"}));
    CHECK(14, git(root, {"switch", "-q", "main"}));
    write(QDir(root).filePath("src/extra.py"), "extra\n");
    CHECK(15, git(root, {"add", "-A"}) && git(root, {"commit", "-q", "-m", "Añade extra"}));
    CHECK(16, git(root, {"merge", "-q", "--no-ff", "-m", "Integra el acceso", "login"}));
    // Pending work: one prepared, one edited, one new, one deleted.
    write(QDir(root).filePath("README.md"), "# demo editado\n");
    CHECK(17, git(root, {"add", "README.md"}));
    write(QDir(root).filePath("src/app.py"), "uno\nDOS\ntres\ncuatro\n");
    write(QDir(root).filePath("notas.txt"), "nueva\n");
    QFile::remove(QDir(root).filePath("src/extra.py"));

    GitPanel panel(root);
    panel.resize(360, 700);
    panel.show();
    CHECK(20, waitFor([&] { return panel.isRepository() && panel.changeCount() == 4 && !panel.refreshing(); }));
    CHECK(21, panel.branchText() == QString("main"));
    GitChangesView *changes = panel.changesView();
    changes->setConfirmations(false);
    CHECK(22, changes->sectionCount("staged") == 1 && changes->sectionCount("changes") == 3 && changes->sectionCount("conflicts") == 0);
    CHECK(23, changes->commitButtonText() == "Guardar (commit) · 1" && !changes->commitEnabled());
    changes->setMessage("Mensaje");
    CHECK(24, changes->commitEnabled());
    changes->setMessage("");

    // Stage, then unstage.
    changes->stagePaths({"src/app.py", "notas.txt"});
    CHECK(30, waitFor([&] { return changes->sectionCount("staged") == 3 && changes->sectionCount("changes") == 1; }));
    changes->unstagePaths({"notas.txt"});
    CHECK(31, waitFor([&] { return changes->sectionCount("staged") == 2 && changes->sectionCount("changes") == 2; }));

    // Discard keeps a recoverable copy and says where it is.
    changes->discardPaths({"notas.txt"});
    CHECK(32, waitFor([&] { return changes->sectionCount("changes") == 1 && panel.toastText().contains("Copia de seguridad") ; }));
    CHECK(33, !QFileInfo::exists(QDir(root).filePath("notas.txt")));
    changes->discardPaths({"src/extra.py"});  // deleted in the working tree: restored
    CHECK(34, waitFor([&] { return QFileInfo::exists(QDir(root).filePath("src/extra.py")) && changes->sectionCount("changes") == 0; }));

    // Commit what is prepared.
    changes->setMessage("Guarda app y README");
    CHECK(35, waitFor([&] { return changes->commitEnabled(); }));
    changes->commitNow();
    CHECK(36, waitFor([&] { return panel.toastText().contains("Guardado en main") && panel.changeCount() == 0; }));
    CHECK(37, changes->emptyStateVisible() && !changes->commitEnabled());

    // Edit again and use "save everything" (nothing prepared).
    write(QDir(root).filePath("src/app.py"), "otra vez\n");
    CHECK(38, waitFor([&] { return changes->sectionCount("changes") == 1; }));
    CHECK(39, changes->commitButtonText() == "Guardar todo (commit)");
    changes->setMessage("Reescribe app");
    changes->commitNow();
    CHECK(40, waitFor([&] { return panel.changeCount() == 0 && panel.toastText().contains("Guardado") ; }));

    // History with the graph.
    panel.showTab(1);
    GitHistoryView *history = panel.historyView();
    CHECK(50, waitFor([&] { return history->commitCount() == 6; }));
    CHECK(51, history->commits().first().subject == "Reescribe app");
    int lanes = 1;
    bool merge = false;
    for (const GitCommit &commit : history->commits()) {
        lanes = qMax(lanes, commit.graph.laneCount);
        merge = merge || commit.parents.size() > 1;
    }
    CHECK(52, merge && lanes >= 2);
    history->selectCommit(1);
    CHECK(53, waitFor([&] { return history->detailText().contains("Guarda app y README") && history->fileCount() == 2; }));
    int mergeRow = -1;
    for (int i = 0; i < history->commits().size(); ++i) if (history->commits().at(i).parents.size() > 1) mergeRow = i;
    CHECK(540, mergeRow >= 0);
    history->selectCommit(mergeRow);  // the merge: shows what it brought in
    CHECK(54, waitFor([&] { return history->detailText().contains("Integra el acceso") && history->fileCount() == 1; }));

    // Branches: create, switch, delete.
    panel.showTab(2);
    GitBranchesView *branches = panel.branchesView();
    branches->setConfirmations(false);
    CHECK(60, waitFor([&] { return branches->branchCount(false) == 2; }));
    branches->createNamed("tarea");
    CHECK(61, waitFor([&] { return panel.branchText() == QString("tarea") && branches->currentBranch() == "tarea"; }));
    branches->switchTo("main", false);
    CHECK(62, waitFor([&] { return panel.branchText() == QString("main"); }));
    branches->deleteNamed("tarea");
    CHECK(63, waitFor([&] { return branches->branchCount(false) == 2 && panel.toastText().contains("borrada"); }));
    branches->switchTo("no-existe", false);
    CHECK(64, waitFor([&] { return !panel.toastText().isEmpty() && !panel.toastText().contains("Ahora estás"); }));


    // --- sync with a remote ------------------------------------------------------------------
    QTemporaryDir remoteDir;
    const QString base = remoteDir.path();
    CHECK(100, git(base, {"init", "-q", "--bare", "-b", "main", "origin.git"}));
    CHECK(101, git(base, {"init", "-q", "-b", "main", "mine"}));
    const QString mine = base + "/mine";
    CHECK(102, git(mine, {"remote", "add", "origin", base + "/origin.git"}));
    write(mine + "/f.txt", "uno\n");
    CHECK(103, git(mine, {"add", "-A"}) && git(mine, {"commit", "-q", "-m", "uno"}));
    GitPanel sync(mine);
    sync.setConfirmations(false);
    sync.resize(360, 600);
    sync.show();
    CHECK(104, waitFor([&] { return sync.isRepository() && sync.syncText() == QString("Publicar rama"); }));
    sync.pushNow();
    CHECK(105, waitFor([&] { return !sync.syncing() && sync.toastText().contains("publicada") && sync.syncText() == QString("Traer novedades"); }));

    CHECK(106, git(base, {"clone", "-q", "origin.git", "other"}));
    const QString other = base + "/other";
    write(other + "/g.txt", "otro\n");
    CHECK(107, git(other, {"add", "-A"}) && git(other, {"commit", "-q", "-m", "otro 1"}) && git(other, {"push", "-q"}));
    sync.fetchNow();
    CHECK(108, waitFor([&] { return !sync.syncing() && sync.syncText() == QString("Recibir 1"); }));
    sync.syncNow();  // the button's default action is to receive
    CHECK(109, waitFor([&] { return !sync.syncing() && sync.syncText() == QString("Traer novedades") && QFileInfo::exists(mine + "/g.txt"); }));
    CHECK(110, sync.toastText().contains("recibidos"));

    write(mine + "/h.txt", "mio\n");
    CHECK(111, git(mine, {"add", "-A"}) && git(mine, {"commit", "-q", "-m", "mío 1"}));
    sync.refresh();
    CHECK(112, waitFor([&] { return sync.syncText() == QString("Enviar 1"); }));
    sync.syncNow();
    CHECK(113, waitFor([&] { return !sync.syncing() && sync.toastText().contains("enviados") && sync.syncText() == QString("Traer novedades"); }));

    // The remote moves on while we also commit: sending is refused with advice, receiving fast-forward is refused, merging works.
    CHECK(119, git(other, {"pull", "-q", "--no-rebase", "--no-edit"}));
    write(other + "/g2.txt", "otro 2\n");
    CHECK(120, git(other, {"add", "-A"}) && git(other, {"commit", "-q", "-m", "otro 2"}) && git(other, {"push", "-q"}));
    write(mine + "/h2.txt", "mio 2\n");
    CHECK(121, git(mine, {"add", "-A"}) && git(mine, {"commit", "-q", "-m", "mío 2"}));
    sync.refresh();
    CHECK(122, waitFor([&] { return sync.syncText() == QString("Enviar 1"); }));
    sync.pushNow();
    CHECK(123, waitFor([&] { return !sync.syncing() && sync.toastText().contains("Recibir") ; }));
    sync.fetchNow();
    CHECK(124, waitFor([&] { return !sync.syncing() && sync.syncText() == QString("Recibir 1"); }));
    sync.pullNow();
    CHECK(125, waitFor([&] { return !sync.syncing() && sync.toastText().contains("línea recta"); }));
    sync.pullNow(GitPullMode::Merge);
    CHECK(126, waitFor([&] { return !sync.syncing() && sync.toastText().contains("recibidos") && sync.syncText() == QString("Enviar 2"); }));
    sync.syncNow();
    CHECK(127, waitFor([&] { return !sync.syncing() && sync.toastText().contains("enviados"); }));

    // A conflict can be abandoned from the panel.
    write(other + "/f.txt", "desde otro\n");
    CHECK(130, git(other, {"pull", "-q", "--no-rebase", "--no-edit"}) && git(other, {"add", "-A"}) && git(other, {"commit", "-q", "-m", "otro edita"}) && git(other, {"push", "-q"}));
    write(mine + "/f.txt", "desde mí\n");
    CHECK(131, git(mine, {"add", "-A"}) && git(mine, {"commit", "-q", "-m", "yo edito"}));
    sync.fetchNow();
    CHECK(132, waitFor([&] { return !sync.syncing() && sync.syncText() == QString("Recibir 1"); }));
    sync.pullNow(GitPullMode::Merge);
    GitChangesView *syncChanges = sync.changesView();
    syncChanges->setConfirmations(false);
    CHECK(133, waitFor([&] { return syncChanges->sectionCount("conflicts") == 1 && syncChanges->abortMergeVisible(); }));
    CHECK(134, !syncChanges->commitEnabled());  // nothing can be saved until the conflict is resolved
    syncChanges->abortMerge();
    CHECK(135, waitFor([&] { return syncChanges->sectionCount("conflicts") == 0 && !syncChanges->abortMergeVisible(); }));
    CHECK(136, read(mine + "/f.txt") == "desde mí\n");
    sync.cancelSync();  // nothing running: harmless

    // Stash: set work aside from the panel, bring it back, drop it.
    write(mine + "/f.txt", "edición a medias\n");
    write(mine + "/borrador.txt", "nuevo\n");
    GitExplorerStatus lastExplorer;
    QObject::connect(&sync, &GitPanel::explorerStatusChanged, [&lastExplorer](const GitExplorerStatus &status) { lastExplorer = status; });
    sync.refresh();
    CHECK(140, waitFor([&] { return syncChanges->sectionCount("changes") == 2; }));
    CHECK(141, waitFor([&] { return lastExplorer.files.value(QDir::cleanPath(sync.repository()->root() + "/f.txt")) == 'M'
                                    && lastExplorer.files.value(QDir::cleanPath(sync.repository()->root() + "/borrador.txt")) == 'N'; }));
    syncChanges->stashChanges("trabajo a medias", true);
    CHECK(142, waitFor([&] { return syncChanges->sectionCount("stashes") == 1 && syncChanges->sectionCount("changes") == 0; }));
    CHECK(143, lastExplorer.isEmpty() || lastExplorer.files.isEmpty());
    syncChanges->applyStash(0);
    CHECK(144, waitFor([&] { return syncChanges->sectionCount("changes") == 2 && syncChanges->sectionCount("stashes") == 1; }));
    CHECK(145, git(mine, {"checkout", "--", "f.txt"}) && QFile::remove(mine + "/borrador.txt"));
    sync.refresh();
    CHECK(146, waitFor([&] { return syncChanges->sectionCount("changes") == 0; }));
    syncChanges->popStash(0);
    CHECK(147, waitFor([&] { return syncChanges->sectionCount("stashes") == 0 && syncChanges->sectionCount("changes") == 2; }));
    syncChanges->stashChanges("", true);
    CHECK(148, waitFor([&] { return syncChanges->sectionCount("stashes") == 1; }));
    syncChanges->dropStash(0);
    CHECK(149, waitFor([&] { return syncChanges->sectionCount("stashes") == 0 && sync.toastText().contains("git stash apply"); }));

    // A repository without a remote says so instead of failing.
    QTemporaryDir lonely;
    CHECK(150, git(lonely.path(), {"init", "-q", "-b", "main"}));
    write(lonely.path() + "/a.txt", "a\n");
    CHECK(151, git(lonely.path(), {"add", "-A"}) && git(lonely.path(), {"commit", "-q", "-m", "a"}));
    GitPanel solo(lonely.path());
    solo.setConfirmations(false);
    CHECK(152, waitFor([&] { return solo.isRepository() && solo.syncText().contains("Sin remoto"); }));
    solo.syncNow();
    solo.pushNow();
    CHECK(153, !solo.syncing());


#ifndef Q_OS_WIN
    // --- the assistant ---------------------------------------------------------------------
    {
        QTemporaryDir tools;
        const QString script = tools.filePath("fake-cli.sh");
        const QString log = tools.filePath("calls.log");
        write(script, QByteArray(R"SH(#!/bin/bash
# Stands in for `chxchx-tech agent chat ... --lean --prompt-file F`: logs what it receives and answers like an agent.
if [ "$1" = "bridge" ] && [ "$2" = "agent-options" ]; then
  for ((i=1; i<=$#; i++)); do [ "${!i}" = "--agent" ] && j=$((i+1)) && who="${!j}"; done
  case "$who" in
    codex) echo '{"schema": "chxchx.agent-options", "schema_version": 1, "default_model": "gpt-grande", "models": [{"id": "gpt-grande", "efforts": ["low", "medium", "high"]}, {"id": "gpt-mini", "efforts": ["low", "medium"]}]}' ;;
    claude) echo '{"schema": "chxchx.agent-options", "schema_version": 1, "default_model": "", "models": [{"id": "sonnet", "efforts": ["low", "high"]}, {"id": "haiku", "efforts": ["low", "high"]}]}' ;;
    *) echo "no disponible" >&2; exit 1 ;;
  esac
  exit 0
fi
prompt_file=""; agent=""
for ((i=1; i<=$#; i++)); do
  [ "${!i}" = "--prompt-file" ] && j=$((i+1)) && prompt_file="${!j}"
  [ "${!i}" = "chat" ] && j=$((i+1)) && agent="${!j}"
done
{ echo "== agent=$agent args=$*"; cat "$prompt_file"; echo; } >> "$FAKE_ASSIST_LOG"
event() { printf '{"schema": "chxchx.agent-event", "schema_version": 1, "type": "%s", %s}\n' "$1" "$2"; }
if [ -n "$FAKE_ASSIST_ERROR" ]; then
  event error '"message": "El proyecto no es confiable."'; event done '"exit_code": 1'; exit 1
fi
if [ -n "$FAKE_ASSIST_SLEEP" ]; then
  read -t 30 line  # waits for the `cancel` line on stdin, as the real CLI does
  event done '"exit_code": 0, "cancelled": true'; exit 0
fi
if [ -n "$FAKE_ASSIST_PLAN" ] && grep -q "Redacta el mensaje de commit" "$prompt_file"; then
  event message '"role": "assistant", "text": "{\"commits\": [{\"message\": \"Mejora greet\", \"files\": [\"app.py\"]}, {\"message\": \"Añade notas\", \"files\": [\"notas.md\", \"no-existe.txt\"]}]}"'
elif grep -q "Redacta el mensaje de commit" "$prompt_file"; then
  event message '"role": "assistant", "text": "```\nMejora greet y añade average.\n\n- Explica el cambio\n```"'
else
  event message '"role": "assistant", "text": "Pasa esto.\n\n1. Abre el archivo.\n2. Pulsa `Preparar`."'
fi
event done '"exit_code": 0'
)SH"));
        QFile::setPermissions(script, QFile::permissions(script) | QFileDevice::ExeUser);
        qputenv("FAKE_ASSIST_LOG", log.toUtf8());

        QTemporaryDir area;
        const QString repo = area.path();
        CHECK(160, git(repo, {"init", "-q", "-b", "main"}));
        write(repo + "/app.py", "def greet():\n    return 'hola'\n");
        write(repo + "/f.txt", "base\n");
        CHECK(161, git(repo, {"add", "-A"}) && git(repo, {"commit", "-q", "-m", "Commit inicial"}));
        write(repo + "/app.py", "def greet():\n    return 'hola de nuevo'\n");
        write(repo + "/.env", "TOKEN=SECRETO_NO_ENVIAR_123456\n");
        write(repo + "/notas.md", "una nota nueva\n");
        CHECK(162, git(repo, {"add", "app.py", ".env"}));

        GitPanel assist(repo);
        assist.setAdviceDialogsEnabled(false);
        assist.resize(380, 800);
        assist.show();
        GitChangesView *changes = assist.changesView();
        CHECK(163, waitFor([&] { return assist.isRepository() && changes->sectionCount("staged") == 2; }));
        GitAssistantBar *bar = changes->assistantBar();

        // Without an agent the card still guides, and says how to get help.
        assist.configureAssistant(script, [] { return QList<GitAssistantAgent>(); }, [] { return QString(); });
        CHECK(164, bar->titleText().contains("2 archivos listos para guardar") && bar->hintText().contains("Conecta un agente"));
        CHECK(165, !bar->adviceEnabled() && !changes->suggestEnabled() && bar->agentText() == "Sin agente");

        // With an agent: suggest a message (the secret file is listed nowhere and its value never leaves).
        QList<GitAssistantAgent> agents = {{"codex", "codex"}};
        QString preferred;
        assist.configureAssistant(script, [&agents] { return agents; }, [&preferred] { return preferred; });
        assist.assistant()->setConsentRequired(false);
        CHECK(166, waitFor([&] { return changes->suggestEnabled() && bar->adviceEnabled() && bar->agentText() == "codex"; }));
        changes->requestSuggestion();
        CHECK(167, waitFor([&] { return bar->busy(); }, 3000) || !changes->messageText().isEmpty());
        CHECK(168, waitFor([&] { return changes->messageText() == "Mejora greet y añade average\n\n- Explica el cambio"; }));
        CHECK(169, changes->assistantNote().contains("Sugerido por codex") && assist.toastText().contains("Mensaje sugerido") && !bar->busy());
        const QString calls = QString::fromUtf8(read(log));
        CHECK(170, calls.contains("agent=codex") && calls.contains("--lean") && calls.contains("--prompt-file") && calls.contains("--cancel-on-stdin"));
        CHECK(171, calls.contains("Redacta el mensaje de commit") && calls.contains("Commit inicial") && calls.contains("+    return 'hola de nuevo'"));
        CHECK(1720, calls.contains("--model gpt-mini") && calls.contains("--effort low"));  // a short question gets the light model, not the biggest
        CHECK(172, !calls.contains("SECRETO_NO_ENVIAR") && calls.contains("Se omitieron por seguridad: .env") && !calls.contains("M  .env"));

        // The person edits the suggestion and saves it: nothing is committed behind their back.
        CHECK(173, git(repo, {"log", "--oneline"}) && QString::fromUtf8(read(repo + "/.git/HEAD")).contains("main"));
        CHECK(174, changes->commitEnabled());

        // Advice: asked on demand, shown, never executed; the question carries the state, not the code.
        assist.adviseNow(GitAdviceKind::WhatNext);
        CHECK(175, waitFor([&] { return assist.lastAdvice().contains("Abre el archivo") && !bar->busy(); }));
        const auto lastCall = [&log] { const QString all = QString::fromUtf8(read(log)); return all.mid(all.lastIndexOf("== agent=")); };
        const QString afterAdvice = lastCall();
        CHECK(176, afterAdvice.contains("No ejecutes comandos") && afterAdvice.contains("Listos para guardar: 2") && !afterAdvice.contains("hola de nuevo"));

        // A failure of the agent is reported plainly, and does not offer advice from the same agent.
        qputenv("FAKE_ASSIST_ERROR", "1");
        changes->requestSuggestion();
        CHECK(177, waitFor([&] { return assist.toastText().contains("no es confiable"); }));
        CHECK(178, !assist.toastOffersAdvice());
        qunsetenv("FAKE_ASSIST_ERROR");

        // Changes of several topics: the assistant proposes one commit for each, prepared one at a time.
        CHECK(300, git(repo, {"reset", "-q"}));
        CHECK(301, waitFor([&] { return changes->sectionCount("staged") == 0 && changes->sectionCount("changes") >= 2; }));
        qputenv("FAKE_ASSIST_PLAN", "1");
        changes->requestSuggestion();
        GitCommitPlanView *plan = assist.commitPlan();
        CHECK(302, waitFor([&] { return plan->groupCount() == 2; }, 8000));
        CHECK(303, plan->plan().at(0).files == QStringList{"app.py"} && plan->plan().at(1).files == QStringList{"notas.md"});  // unknown files are dropped
        CHECK(304, waitFor([&] { return changes->sectionCount("staged") == 1 && changes->messageText() == "Mejora greet"; }));
        plan->prepare(1);  // another group: the first one is unprepared, this one is prepared
        CHECK(305, waitFor([&] { return changes->sectionCount("staged") == 1 && changes->messageText() == "Añade notas"; }));
        CHECK(306, QString::fromUtf8(gitOutput(repo, {"diff", "--cached", "--name-only"})).trimmed() == "notas.md");
        changes->commitNow();
        CHECK(307, waitFor([&] { return plan->groupCount() == 1 && plan->plan().first().files == QStringList{"app.py"}; }, 8000));  // committed files leave the plan
        qunsetenv("FAKE_ASSIST_PLAN");

        // A Git error offers to ask for advice; the answer uses the error.
        assist.branchesView()->switchTo("no-existe", false);
        CHECK(179, waitFor([&] { return assist.toastOffersAdvice(); }));
        CHECK(180, assist.toastText().contains("no-existe") || !assist.toastText().isEmpty());

        // Cancelling stops a slow agent promptly and leaves the assistant usable.
        qputenv("FAKE_ASSIST_SLEEP", "1");
        changes->requestSuggestion();
        CHECK(181, waitFor([&] { return bar->busy(); }, 5000));
        QElapsedTimer cancelClock;
        cancelClock.start();
        assist.assistant()->cancel();
        CHECK(182, waitFor([&] { return !bar->busy(); }, 3000) && cancelClock.elapsed() < 3000);
        qunsetenv("FAKE_ASSIST_SLEEP");
        changes->requestSuggestion();
        CHECK(183, waitFor([&] { return changes->assistantNote().contains("Sugerido por codex"); }));

        // The person can switch agents; the next question goes to the chosen one.
        agents = {{"codex", "codex"}, {"claude", "claude"}};
        assist.assistant()->chooseAgent("claude");
        CHECK(184, waitFor([&] { return bar->agentText() == "claude"; }));
        changes->setMessage("");
        changes->requestSuggestion();
        CHECK(185, waitFor([&] { return QString::fromUtf8(read(log)).contains("agent=claude"); }));
        CHECK(186, waitFor([&] { return !bar->busy(); }));
        preferred = "codex";  // the chat in front of the person wins over the saved choice only if nothing was chosen
        CHECK(187, assist.assistant()->currentAgentId() == "claude");
        assist.assistant()->chooseAgent("");
        CHECK(188, assist.assistant()->currentAgentId() == "codex");

        // Which model answers: automatic is a light one; the person can pick any, or the agent's own default.
        const auto callCount = [&log] { return QString::fromUtf8(read(log)).count("== agent="); };
        const auto ask = [&] {
            const int before = callCount();
            changes->setMessage("");
            changes->requestSuggestion();
            return waitFor([&] { return callCount() > before && !bar->busy(); });
        };
        assist.assistant()->chooseAgent("codex");
        CHECK(2000, waitFor([&] { return bar->modelChoices().size() >= 4; }));
        const QStringList choices = bar->modelChoices();
        CHECK(2001, choices.value(0) == "Automático (ligero): gpt-mini" && choices.contains("El predeterminado del agente") && choices.contains("gpt-grande") && choices.contains("gpt-mini"));
        CHECK(2002, bar->modelPreferenceText().startsWith("Automático"));
        assist.assistant()->setModelPreference("gpt-grande");
        CHECK(2003, waitFor([&] { return bar->modelPreferenceText() == "gpt-grande"; }));
        CHECK(2004, ask() && lastCall().contains("--model gpt-grande") && lastCall().contains("--effort low"));
        assist.assistant()->setModelPreference(GitAgentDefaultModel);
        CHECK(2005, waitFor([&] { return bar->modelPreferenceText() == "El predeterminado del agente"; }));
        CHECK(2006, ask() && !lastCall().contains("--model") && lastCall().contains("--effort low"));  // the agent's model, with its lowest effort
        assist.assistant()->setModelPreference("luna-6");  // a model the list does not show: used as typed, with no effort guess
        CHECK(2007, ask() && lastCall().contains("--model luna-6") && !lastCall().contains("--effort"));
        assist.assistant()->setModelPreference("");
        CHECK(2008, waitFor([&] { return bar->modelPreferenceText().startsWith("Automático"); }));
        CHECK(2009, ask() && lastCall().contains("--model gpt-mini") && lastCall().contains("--effort low"));
        agents = {{"codex", "codex"}, {"claude", "claude"}, {"opencode", "opencode"}};
        assist.assistant()->chooseAgent("claude");
        CHECK(2010, waitFor([&] { return bar->agentText() == "claude" && bar->modelChoices().contains("haiku"); }));
        CHECK(2011, ask() && lastCall().contains("agent=claude") && lastCall().contains("--model haiku") && !lastCall().contains("--effort"));  // haiku takes no effort
        assist.assistant()->chooseAgent("opencode");  // its model list cannot be read: the agent decides
        CHECK(2012, waitFor([&] { return bar->agentText() == "opencode"; }));
        CHECK(2013, ask() && lastCall().contains("agent=opencode") && !lastCall().contains("--model") && !lastCall().contains("--effort"));
        assist.assistant()->chooseAgent("codex");
        CHECK(2014, waitFor([&] { return bar->agentText() == "codex"; }));

        // Conflicts: the card changes, and the advice question carries the conflicting blocks.
        CHECK(190, git(repo, {"add", "-A"}) && git(repo, {"commit", "-q", "-m", "Guarda lo pendiente"}));
        CHECK(191, git(repo, {"switch", "-q", "-c", "otra"}));
        write(repo + "/f.txt", "desde otra\n");
        CHECK(192, git(repo, {"commit", "-q", "-am", "otra edita"}) && git(repo, {"switch", "-q", "main"}));
        write(repo + "/f.txt", "desde main\n");
        CHECK(193, git(repo, {"commit", "-q", "-am", "main edita"}));
        git(repo, {"merge", "-q", "otra"});  // conflicts, which is the point
        assist.refresh();
        CHECK(194, waitFor([&] { return changes->sectionCount("conflicts") == 1 && bar->titleText().contains("1 archivo en conflicto"); }));
        CHECK(195, bar->stepsText().contains("Cancelar la fusión") || bar->stepsText().contains("marcas"));
        bar->requestAdvice();
        CHECK(196, waitFor([&] { return QString::fromUtf8(read(log)).contains("Archivos en conflicto: f.txt"); }));
        const QString conflictCalls = lastCall();
        CHECK(197, conflictCalls.contains("<<<<<<< HEAD") && conflictCalls.contains("desde otra") && conflictCalls.contains("fusión (merge) a medias"));
        CHECK(198, !changes->suggestEnabled());  // no message can be suggested until the conflict is resolved
        assist.changesView()->setConfirmations(false);
        changes->abortMerge();
        CHECK(199, waitFor([&] { return changes->sectionCount("conflicts") == 0; }));
    }
#endif

    // --- diff view -------------------------------------------------------------------------
    write(QDir(root).filePath("src/app.py"), "otra vez\nmás\n");
    write(QDir(root).filePath("nuevo.txt"), "a\nb\nc\n");
    write(QDir(root).filePath("imagen.bin"), QByteArray("PNG\0\1\2", 6));
    GitRepository *repo = panel.repository();
    GitDiffRequest edited;
    edited.path = "src/app.py";
    GitDiffView diffView(repo, edited);
    diffView.resize(900, 500);
    diffView.show();
    CHECK(70, waitFor([&] { return diffView.visibleLineCount() > 1 && diffView.statusText().contains("añadida"); }));
    CHECK(71, diffView.paneCount() == 2 && diffView.sideBySide());
    diffView.setSideBySide(false);
    CHECK(72, diffView.paneCount() == 1 && diffView.visibleLineCount() == 3);
    diffView.setSideBySide(true);
    CHECK(73, diffView.paneCount() == 2);
    GitDiffRequest fresh;
    fresh.source = GitDiffRequest::Source::Untracked;
    fresh.path = "nuevo.txt";
    GitDiffView freshView(repo, fresh);
    CHECK(74, freshView.statusText().contains("3 líneas añadidas") && freshView.visibleLineCount() == 4);
    GitDiffRequest binary;
    binary.source = GitDiffRequest::Source::Untracked;
    binary.path = "imagen.bin";
    GitDiffView binaryView(repo, binary);
    CHECK(75, binaryView.statusText().contains("binario"));
    GitDiffRequest commit;
    commit.source = GitDiffRequest::Source::Commit;
    commit.hash = history->commits().first().hash;
    commit.path = "src/app.py";
    GitDiffView commitView(repo, commit);
    CHECK(76, waitFor([&] { return commitView.statusText().contains("Commit ") ; }));
    GitDiffRequest missing;
    missing.path = "no/existe.txt";
    GitDiffView emptyView(repo, missing);
    CHECK(77, waitFor([&] { return emptyView.statusText().contains("Sin cambios"); }));

    // Closing a diff tab while git is still working must not crash.
    QPointer<GitDiffView> closing = new GitDiffView(repo, edited);
    delete closing.data();
    CHECK(78, waitFor([&] { return !repo->isBusy(); }));
    CHECK(79, closing.isNull());

    // The panel can also be destroyed with work in flight.
    {
        auto *transient = new GitPanel(root);
        transient->show();
        QCoreApplication::processEvents();
        delete transient;
        QElapsedTimer settle;
        settle.start();
        while (settle.elapsed() < 400) QCoreApplication::processEvents(QEventLoop::AllEvents, 20);
    }
    return 0;
}
