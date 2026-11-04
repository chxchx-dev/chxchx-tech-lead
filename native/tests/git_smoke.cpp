// Parsers from literal text, then GitRepository against a real temporary repository.
#include "../src/git/git_assistant.hpp"
#include "../src/git/git_assistant_models.hpp"
#include "../src/git/git_assistant_prompts.hpp"
#include "../src/git/git_explorer.hpp"
#include "../src/git/git_guide.hpp"
#include "../src/git/git_parsers.hpp"
#include "../src/git/git_repository.hpp"

#include <QCoreApplication>
#include <QDir>
#include <QElapsedTimer>
#include <QFile>
#include <QProcess>
#include <QTemporaryDir>

#include <cstdio>

namespace {

int fail(int code, const char *what)
{
    std::fprintf(stderr, "git smoke %d: %s\n", code, what);
    return code;
}

#define CHECK(code, condition) do { if (!(condition)) return fail(code, #condition); } while (0)

void writeFile(const QString &path, const QByteArray &content)
{
    QDir().mkpath(QFileInfo(path).absolutePath());
    QFile file(path);
    if (file.open(QIODevice::WriteOnly)) file.write(content);
}

QByteArray nul(std::initializer_list<const char *> parts)
{
    QByteArray result;
    for (const char *part : parts) {
        result += part;
        result.append('\0');
    }
    return result;
}

void waitIdle(const GitRepository &repository)
{
    QElapsedTimer timer;
    timer.start();
    while (repository.isBusy() && timer.elapsed() < 20000) QCoreApplication::processEvents(QEventLoop::AllEvents, 20);
    QCoreApplication::processEvents();
}


int assistantChecks()
{
    // Files that may hold secrets are recognized; ordinary ones are not.
    for (const char *path : {".env", "config/.env.local", "id_rsa", "keys/server.pem", "app/prod.key", ".ssh/config", "src/secrets.py", "creds/credentials.json", ".aws/credentials", ".npmrc"}) {
        if (!isSensitivePath(path)) return fail(300, path);
    }
    for (const char *path : {"src/app.py", "docs/environment.md", "README.md", "src/keyboard.py", "src/monkey.cpp", "env/readme.txt"}) {
        if (isSensitivePath(path)) return fail(301, path);
    }
    // Values that look like secrets are masked; ordinary code is untouched.
    const QString masked = redactSecrets("password = \"hunter2hunter2\"\nAPI_KEY=abcdef123456\nurl = 'x'\ntoken_count = 5\nghp_abcdefghijklmnopqrstuvwxyz0123\n-----BEGIN RSA PRIVATE KEY-----\nMIIB\n-----END RSA PRIVATE KEY-----\nk = AKIAABCDEFGHIJKLMNOP");
    CHECK(302, !masked.contains("hunter2") && !masked.contains("abcdef123456") && !masked.contains("ghp_") && !masked.contains("MIIB") && !masked.contains("AKIA"));
    CHECK(303, masked.contains("password = \"[omitido]\"") && masked.contains("API_KEY=[omitido]") && masked.contains("token_count = 5") && masked.contains("url = 'x'"));
    const QString diff = "diff --git a/src/app.py b/src/app.py\n--- a/src/app.py\n+++ b/src/app.py\n@@ -1 +1 @@\n-x\n+y = compute()\n"
                         "diff --git a/.env b/.env\n--- a/.env\n+++ b/.env\n@@ -1 +1 @@\n-A=1\n+DB_URL=postgres://admin:supersecret@host/db\n"
                         "diff --git a/src/b.py b/src/b.py\n--- a/src/b.py\n+++ b/src/b.py\n@@ -1 +1 @@\n-o\n+secret = \"valor-muy-secreto\"\n";
    QStringList omitted;
    const QString safe = redactDiff(diff, &omitted);
    CHECK(304, omitted == QStringList({".env"}) && safe.contains("archivo sensible omitido: .env") && !safe.contains("supersecret") && !safe.contains("DB_URL"));
    CHECK(305, safe.contains("+y = compute()") && safe.contains("diff --git a/src/b.py") && !safe.contains("valor-muy-secreto") && safe.contains("secret = \"[omitido]\""));
    bool truncated = false;
    CHECK(306, truncateForPrompt("abcdef", 3, &truncated).startsWith("abc") && truncated && truncateForPrompt("abc", 3, &truncated) == "abc" && !truncated);

    const QString conflicted = "antes\n<<<<<<< HEAD\nA1\nA2\n=======\nB1\n>>>>>>> otra\nmedio\n<<<<<<< HEAD\nC\n=======\nD\n>>>>>>> otra\nfin\n";
    const QString hunks = extractConflictHunks(conflicted, 1);
    CHECK(307, hunks.contains("A1") && hunks.contains(">>>>>>> otra") && !hunks.contains("\nC\n") && !hunks.contains("antes"));
    CHECK(308, extractConflictHunks(conflicted, 5).contains("\nD\n") && extractConflictHunks("sin conflictos\n").isEmpty());
    CHECK(309, extractConflictHunks("<<<<<<< a\n1\n2\n3\n4\n5\n6\n=======\nx\n>>>>>>> b\n", 1, 4).contains("recortado"));
    const QString fresh = newFileDiff("docs/n.md", "uno\ndos\n");
    CHECK(310, fresh.contains("diff --git a/docs/n.md b/docs/n.md") && fresh.contains("@@ -0,0 +1,2 @@") && fresh.contains("+dos"));
    CHECK(311, redactDiff(newFileDiff(".env", "TOKEN=abcdefghijk\n")).contains("archivo sensible omitido"));

    // The commit prompt: rules, style, only safe files, redacted and bounded diff.
    GitAssistContext context;
    context.status = parseStatusV2(nul({"# branch.oid x", "# branch.head feature/login", "1 M. N... 100644 100644 100644 a b src/app.py", "1 M. N... 100644 100644 100644 a b .env", "1 .M N... 100644 100644 100644 a b notas.md"}));
    context.recentSubjects = {"Añade la carga de configuración", "Corrige el acceso"};
    context.diff = diff;
    const QString commit = buildCommitPrompt(context);
    CHECK(312, commit.contains("SOLO con el mensaje") && commit.contains("72 caracteres") && commit.contains("feature/login"));
    CHECK(313, commit.contains("- Añade la carga de configuración") && commit.contains("M  src/app.py") && !commit.contains("M  .env") && commit.contains("los preparados"));
    CHECK(314, !commit.contains("supersecret") && commit.contains("Se omitieron por seguridad: .env") && !commit.contains("notas.md"));
    context.status = parseStatusV2(nul({"# branch.oid x", "# branch.head main", "1 .M N... 100644 100644 100644 a b notas.md"}));
    CHECK(315, buildCommitPrompt(context).contains("todo lo pendiente") && buildCommitPrompt(context).contains(" M notas.md"));
    context.diff = QString(60000, 'x');
    CHECK(316, buildCommitPrompt(context).contains("recortada") && buildCommitPrompt(context).size() < 52000);

    // Advice prompts carry the situation, the state and (for conflicts) the blocks, never executing anything.
    GitAssistContext trouble;
    trouble.status = parseStatusV2(nul({"# branch.oid x", "# branch.head main", "# branch.upstream origin/main", "# branch.ab +2 -3", "u UU N... 1 2 3 4 a b c src/a.py"}));
    trouble.conflictFiles = {"src/a.py"};
    trouble.conflictExcerpts = conflicted + "\npassword = \"valor-secreto-123\"";
    trouble.mergeInProgress = true;
    trouble.stashCount = 1;
    trouble.lastError = "CONFLICT (content): Merge conflict in src/a.py";
    const QString conflictsAdvice = buildAdvicePrompt(GitAdviceKind::Conflicts, trouble);
    CHECK(317, conflictsAdvice.contains("No ejecutes comandos") && conflictsAdvice.contains("Markdown") && conflictsAdvice.contains("Archivos en conflicto: src/a.py"));
    CHECK(318, conflictsAdvice.contains("<<<<<<< HEAD") && !conflictsAdvice.contains("valor-secreto-123") && conflictsAdvice.contains("2 por enviar, 3 por recibir"));
    CHECK(319, conflictsAdvice.contains("fusión (merge) a medias") && conflictsAdvice.contains("Merge conflict in src/a.py") && conflictsAdvice.contains("Apartados (stash): 1"));
    CHECK(320, buildAdvicePrompt(GitAdviceKind::PushRejected, trouble).contains("sin forzar") && buildAdvicePrompt(GitAdviceKind::Diverged, trouble).contains("rebase"));
    CHECK(321, buildAdvicePrompt(GitAdviceKind::Credentials, trouble).contains("sin pedir ni mostrar secretos") && !adviceTitle(GitAdviceKind::Failure).isEmpty());

    // The agent's answer becomes a commit message.
    CHECK(322, cleanCommitMessage("```\nCorrige el login\n\n- Valida campos vacíos\n```") == "Corrige el login\n\n- Valida campos vacíos");
    CHECK(323, cleanCommitMessage("Mensaje de commit: \"Añade average.\"") == "Añade average");
    CHECK(324, cleanCommitMessage("  `Mejora greet`.  \n") == "Mejora greet" && cleanCommitMessage("Título.\n\n- uno.") == "Título\n\n- uno.");

    // What the CLI printed becomes text or an error.
    const auto event = [](const char *type, const char *key, const char *value) {
        return QByteArray("{\"schema\": \"chxchx.agent-event\", \"schema_version\": 1, \"type\": \"") + type + "\", \"" + key + "\": \"" + value + "\"}\n";
    };
    GitAssistantResult parsed = GitAssistant::parseOutput(event("session", "session_id", "s") + event("message", "text", "Hola") + "ruido\n" + event("message", "text", "Mundo")
                                                          + "{\"schema\": \"otro\", \"type\": \"message\", \"text\": \"ajeno\"}\n", 0, "");
    CHECK(325, parsed.ok() && parsed.text == "Hola\n\nMundo");
    parsed = GitAssistant::parseOutput(event("error", "message", "El proyecto no es confiable."), 1, "");
    CHECK(326, !parsed.ok() && parsed.error == "El proyecto no es confiable." && parsed.text.isEmpty());
    parsed = GitAssistant::parseOutput("", 3, "boom");
    CHECK(327, !parsed.ok() && parsed.error == "boom" && GitAssistant::parseOutput("", 3, "").error.contains("código 3"));
    CHECK(328, !GitAssistant::parseOutput("", 0, "").ok());  // an empty answer is not a success

    // The rule-based guide decides the next step from the state alone.
    const auto status = [](std::initializer_list<const char *> parts) {
        QByteArray all = nul({"# branch.oid x", "# branch.head main"});
        all += nul(parts);
        return parseStatusV2(all);
    };
    GitGuideFacts facts;
    facts.hasRemotes = true;
    GitGuidance guide = guidanceFor(status({"u UU N... 1 2 3 4 a b c f.txt"}), facts);
    CHECK(329, guide.tone == GitGuidance::Tone::Warning && guide.adviceKind == GitAdviceKind::Conflicts && guide.title.contains("1 archivo en conflicto") && guide.steps.size() == 4);
    facts.mergeInProgress = true;
    CHECK(330, guidanceFor(status({"u UU N... 1 2 3 4 a b c f.txt"}), facts).steps.last().contains("Cancelar la fusión"));
    facts.mergeInProgress = false;
    guide = guidanceFor(status({"1 M. N... 100644 100644 100644 a b a.py"}), facts);
    CHECK(331, guide.icon == "git-commit" && guide.offersMessageSuggestion && guide.title.contains("listo para guardar"));
    guide = guidanceFor(status({"1 .M N... 100644 100644 100644 a b a.py", "? n.txt"}), facts);
    CHECK(332, guide.title.contains("2 cambios sin guardar") && guide.offersMessageSuggestion && guide.steps.size() == 3);
    GitStatus diverged = parseStatusV2(nul({"# branch.oid x", "# branch.head main", "# branch.upstream origin/main", "# branch.ab +2 -3"}));
    guide = guidanceFor(diverged, facts);
    CHECK(333, guide.adviceKind == GitAdviceKind::Diverged && guide.tone == GitGuidance::Tone::Warning && guide.steps.first().contains("3 commit(s) nuevos") && guide.steps.first().contains("2 tuyos"));
    GitStatus behind = parseStatusV2(nul({"# branch.oid x", "# branch.head main", "# branch.upstream origin/main", "# branch.ab +0 -4"}));
    CHECK(334, guidanceFor(behind, facts).icon == "git-pull" && guidanceFor(behind, facts).title.contains("4 novedad"));
    GitStatus ahead = parseStatusV2(nul({"# branch.oid x", "# branch.head main", "# branch.upstream origin/main", "# branch.ab +1 -0"}));
    CHECK(335, guidanceFor(ahead, facts).icon == "git-push");
    GitStatus unpublished = parseStatusV2(nul({"# branch.oid abc", "# branch.head tarea"}));
    CHECK(336, guidanceFor(unpublished, facts).title.contains("solo existe en tu equipo"));
    facts.hasRemotes = false;
    CHECK(337, guidanceFor(unpublished, facts).tone == GitGuidance::Tone::Done);  // nothing to publish to
    facts.stashCount = 2;
    CHECK(338, guidanceFor(unpublished, facts).steps.last().contains("2 apartado"));
    CHECK(339, guidanceFor(parseStatusV2(nul({"# branch.oid (initial)", "# branch.head main"})), facts).title.contains("Todavía no hay commits"));

    // Light models: by name or by size; large ones and unknown names are not light.
    for (const char *id : {"gpt-5-mini", "gpt-nano", "haiku", "gemini-flash", "model-lite", "ollama/qwen2.5-coder:7b-ctx32k", "llama3:8b", "phi:1.5b", "anthropic/claude-3-5-haiku"}) {
        if (!looksLight(id)) return fail(340, id);
    }
    for (const char *id : {"gpt-5", "gpt-5-codex", "opus", "sonnet", "ollama/qwen3-coder:30b-ctx32k", "gpt-oss:120b", "gpt-oss:20b", "luna-6", "devstral:24b"}) {
        if (looksLight(id)) return fail(341, id);
    }
    CHECK(342, lowEffort({"medium", "low", "high"}) == "low" && lowEffort({"minimal", "high"}) == "minimal" && lowEffort({"high"}).isEmpty() && lowEffort({}).isEmpty());
    const GitModelList codex = parseAgentOptions(R"({"schema": "chxchx.agent-options", "default_model": "gpt-grande", "models": [
        {"id": "gpt-grande", "efforts": ["low", "medium", "high"]}, {"id": "gpt-mini", "efforts": ["medium", "low"]}]})");
    CHECK(343, codex.loaded && codex.models.size() == 2 && codex.defaultModel == "gpt-grande" && codex.models[1].efforts == QStringList({"medium", "low"}));
    CHECK(344, !parseAgentOptions("{}").loaded && !parseAgentOptions("no es json").loaded && parseAgentOptions("{}").models.isEmpty());
    GitModelChoice choice = chooseModel(codex, "");
    CHECK(345, choice.model == "gpt-mini" && choice.effort == "low" && choice.light);
    choice = chooseModel(codex, GitAgentDefaultModel);
    CHECK(346, choice.model.isEmpty() && choice.effort == "low" && !choice.light);  // the agent's model, with the lowest effort it takes
    choice = chooseModel(codex, "gpt-grande");
    CHECK(347, choice.model == "gpt-grande" && choice.effort == "low" && !choice.light);
    choice = chooseModel(codex, "luna-6");  // a model the person typed or picked that the list does not know
    CHECK(348, choice.model == "luna-6" && choice.effort.isEmpty());
    const GitModelList plain = parseAgentOptions(R"({"schema": "chxchx.agent-options", "default_model": "", "models": [{"id": "opus", "efforts": ["low", "high"]}, {"id": "sonnet", "efforts": ["low"]}]})");
    choice = chooseModel(plain, "");
    CHECK(349, choice.model.isEmpty() && choice.effort.isEmpty() && !choice.light);  // nothing light: the agent decides
    const GitModelList claude = parseAgentOptions(R"({"schema": "chxchx.agent-options", "default_model": "", "models": [{"id": "sonnet", "efforts": ["low"]}, {"id": "haiku", "efforts": ["low", "high"]}]})");
    choice = chooseModel(claude, "");
    CHECK(350, choice.model == "haiku" && choice.effort.isEmpty() && choice.light);  // haiku takes no effort setting
    CHECK(351, describeChoice(chooseModel(codex, "")) == "gpt-mini · esfuerzo bajo" && describeChoice(chooseModel(plain, "")) == "modelo del agente");
    return 0;
}

int parserChecks()
{
    const GitStatus status = parseStatusV2(nul({
        "# branch.oid 1234567890abcdef", "# branch.head main", "# branch.upstream origin/main", "# branch.ab +2 -1",
        "1 .M N... 100644 100644 100644 aaa bbb src/a file.cpp",
        "1 A. N... 000000 100644 100644 000 ccc new.txt",
        "2 R. N... 100644 100644 100644 ddd eee R100 renamed.txt", "old name.txt",
        "u UU N... 100644 100644 100644 100644 f1 f2 f3 conflict.txt",
        "? untracked dir/file.txt", "! ignored.log"}));
    CHECK(1, status.head == "main" && status.upstream == "origin/main" && status.ahead == 2 && status.behind == 1);
    CHECK(2, status.entries.size() == 5);
    CHECK(3, status.entries[0].path == "src/a file.cpp" && status.entries[0].unstaged() && !status.entries[0].staged());
    CHECK(4, status.entries[1].staged() && status.entries[1].index == 'A' && !status.entries[1].unstaged());
    CHECK(5, status.entries[2].kind == GitEntry::Kind::Renamed && status.entries[2].path == "renamed.txt" && status.entries[2].origPath == "old name.txt");
    CHECK(6, status.entries[3].conflicted() && status.entries[3].path == "conflict.txt");
    CHECK(7, status.entries[4].untracked() && status.entries[4].path == "untracked dir/file.txt");
    CHECK(8, status.stagedEntries().size() == 2 && status.changedEntries().size() == 2 && status.conflicts().size() == 1);
    const GitStatus fresh = parseStatusV2(nul({"# branch.oid (initial)", "# branch.head master"}));
    CHECK(9, fresh.initial && fresh.head == "master" && fresh.entries.isEmpty());
    const GitStatus detached = parseStatusV2(nul({"# branch.oid abc", "# branch.head (detached)"}));
    CHECK(10, detached.detached && detached.head.isEmpty());

    const QByteArray sep(1, '\x1f');
    const QList<GitBranch> branches = parseBranches(QByteArray("*") + sep + "refs/heads/main" + sep + "origin/main" + sep + "ahead 1" + sep + "abc1234" + sep + "Subject: x\n"
        + " " + sep + "refs/remotes/origin/main" + sep + sep + sep + "abc1234" + sep + "Subject: x\n"
        + " " + sep + "refs/remotes/origin/HEAD" + sep + sep + sep + "abc1234" + sep + "Subject: x\n");
    CHECK(11, branches.size() == 2 && branches[0].current && !branches[0].remote && branches[0].name == "main" && branches[0].track == "ahead 1");
    CHECK(12, branches[1].remote && branches[1].name == "origin/main");

    QList<GitCommit> log = parseLog(QByteArray("h3") + sep + "h1 h2" + sep + "Ana" + sep + "1700000000" + sep + "HEAD -> main, tag: v1" + sep + "Merge" + QByteArray(1, '\0')
        + "h2" + sep + "h0" + sep + "Beto" + sep + "1699990000" + sep + sep + "Rama" + QByteArray(1, '\0')
        + "h1" + sep + "h0" + sep + "Ana" + sep + "1699980000" + sep + "origin/main" + sep + "Principal" + QByteArray(1, '\0')
        + "h0" + sep + sep + "Ana" + sep + "1699970000" + sep + sep + "Inicial" + QByteArray(1, '\0'));
    CHECK(13, log.size() == 4 && log[0].parents == QStringList({"h1", "h2"}) && log[0].refs == QStringList({"HEAD -> main", "tag: v1"}));
    CHECK(14, log[0].shortHash() == "h3" && log[3].parents.isEmpty());
    QVector<QString> lanes;
    assignGraph(log, lanes);
    CHECK(15, log[0].graph.lane == 0 && !log[0].graph.incoming && log[0].graph.parentLanes == QVector<int>({0, 1}));
    CHECK(16, log[1].graph.lane == 1 && log[1].graph.through == QVector<int>({0}));  // h2 on its own lane, h1's line passes
    CHECK(17, log[2].graph.lane == 0 && log[2].graph.through == QVector<int>({1}));
    CHECK(18, log[3].graph.lane == 0 && log[3].graph.mergeIn == QVector<int>({1}) && lanes.isEmpty());

    const GitDiff diff = parseUnifiedDiff(
        "diff --git a/f b/f\nindex 1..2 100644\n--- a/f\n+++ b/f\n@@ -1,4 +1,5 @@ func\n keep\n-old one\n-old two\n+new one\n+new two\n+extra\n tail\n\\ No newline at end of file\n"
        "@@ -20 +21 @@\n-x\n+y\n");
    CHECK(19, diff.hunkRows.size() == 2 && diff.additions == 4 && diff.deletions == 3);
    CHECK(20, diff.rows[1].kind == GitDiffRow::Kind::Context && diff.rows[1].leftNumber == 1 && diff.rows[1].rightNumber == 1);
    CHECK(21, diff.rows[2].kind == GitDiffRow::Kind::Changed && diff.rows[2].left == "old one" && diff.rows[2].right == "new one" && diff.rows[2].leftNumber == 2);
    CHECK(22, diff.rows[4].kind == GitDiffRow::Kind::Added && diff.rows[4].right == "extra" && diff.rows[4].rightNumber == 4);
    CHECK(23, diff.rows[6].kind == GitDiffRow::Kind::Note && diff.rows[7].kind == GitDiffRow::Kind::Hunk);
    CHECK(24, diff.rows[8].kind == GitDiffRow::Kind::Changed && diff.rows[8].leftNumber == 20 && diff.rows[8].rightNumber == 21);
    CHECK(25, parseUnifiedDiff("diff --git a/i.png b/i.png\nBinary files a/i.png and b/i.png differ\n").binary);
    const GitDiff fresh2 = diffForNewFile("a\nb\n");
    CHECK(26, fresh2.additions == 2 && fresh2.rows.size() == 3 && fresh2.rows[2].rightNumber == 2);

    const QList<GitFileChange> files = parseNameStatus(nul({"M", "a.cpp", "R100", "old.txt", "new.txt", "D", "gone.txt"}));
    CHECK(27, files.size() == 3 && files[1].status == "R" && files[1].origPath == "old.txt" && files[1].path == "new.txt" && files[2].status == "D");
    const QList<GitStash> stashes = parseStashList(QByteArray("stash@{0}") + sep + "1700000000" + sep + "On main: arreglo a medias\n"
        + "stash@{1}" + sep + "1699990000" + sep + "WIP on feature/x: abc123 mensaje: con dos puntos\n" + "basura\n");
    CHECK(128, stashes.size() == 2 && stashes[0].index == 0 && stashes[0].message == "On main: arreglo a medias" && stashes[1].index == 1);
    CHECK(129, stashes[1].message.contains("feature/x") && stashes[1].time.toSecsSinceEpoch() == 1699990000);

    GitStatus changed = parseStatusV2(nul({"# branch.oid x", "# branch.head main",
        "1 .M N... 100644 100644 100644 a b src/deep/a.cpp", "1 A. N... 000000 100644 100644 0 c src/new.cpp",
        "u UU N... 1 2 3 4 a b c conflict.txt", "? docs/notes/todo.md"}));
    const GitExplorerStatus explorer = buildExplorerStatus(changed, "/repo/");
    CHECK(130, explorer.files.value("/repo/src/deep/a.cpp") == 'M' && explorer.files.value("/repo/src/new.cpp") == 'A');
    CHECK(131, explorer.files.value("/repo/conflict.txt") == 'U' && explorer.files.value("/repo/docs/notes/todo.md") == 'N');
    CHECK(132, explorer.folders.contains("/repo/src") && explorer.folders.contains("/repo/src/deep") && explorer.folders.contains("/repo/docs")
              && explorer.folders.contains("/repo/docs/notes") && !explorer.folders.contains("/repo") && explorer.folders.size() == 4);
    CHECK(133, buildExplorerStatus(GitStatus(), "/repo").isEmpty());

    const auto problem = [](const char *error) { GitResult r; r.exitCode = 1; r.err = error; return r; };
    CHECK(134, problem("fatal: Authentication failed for 'https://x'").problem() == GitProblem::Credentials);
    CHECK(135, problem("git@host: Permission denied (publickey).").problem() == GitProblem::Credentials);
    CHECK(136, problem("fatal: could not read Username for 'https://x': terminal prompts disabled").problem() == GitProblem::Credentials);
    CHECK(137, problem("Host key verification failed.").problem() == GitProblem::Credentials);
    CHECK(138, problem(" ! [rejected]  main -> main (fetch first)").problem() == GitProblem::Rejected);
    CHECK(139, problem("hint: Updates were rejected because the tip of your current branch is behind").problem() == GitProblem::Rejected);
    CHECK(140, problem("fatal: Not possible to fast-forward, aborting.").problem() == GitProblem::Diverged);
    CHECK(141, problem("fatal: The current branch topic has no upstream branch.").problem() == GitProblem::NoUpstream);
    CHECK(142, problem("fatal: No configured push destination.").problem() == GitProblem::NoRemote);
    CHECK(143, problem("Please tell me who you are").problem() == GitProblem::Identity);
    CHECK(144, problem("algo raro").problem() == GitProblem::Other && problem("algo raro").message() == "algo raro");
    CHECK(145, problem("Authentication failed").message().contains("credenciales") && problem("[rejected]").message().contains("Recibir"));
    GitResult good;
    good.exitCode = 0;
    CHECK(146, good.problem() == GitProblem::None && good.ok());
    GitResult cancelled;
    cancelled.exitCode = 0;
    cancelled.cancelled = true;
    CHECK(147, !cancelled.ok() && cancelled.problem() == GitProblem::Cancelled && cancelled.message() == "Cancelado.");
    return assistantChecks();
}

// --- real repository ------------------------------------------------------------------------

struct Fixture {
    QTemporaryDir dir;
    QTemporaryDir home;
    GitRepository *repo = nullptr;

    Fixture()
    {
        qputenv("HOME", home.path().toUtf8());
        qputenv("USERPROFILE", home.path().toUtf8());
        qputenv("GIT_CONFIG_NOSYSTEM", "1");
        qputenv("GIT_CONFIG_GLOBAL", QByteArray(home.filePath(".gitconfig").toUtf8()));
        qputenv("GIT_AUTHOR_NAME", "Ana");
        qputenv("GIT_AUTHOR_EMAIL", "ana@example.com");
        qputenv("GIT_COMMITTER_NAME", "Ana");
        qputenv("GIT_COMMITTER_EMAIL", "ana@example.com");
        repo = new GitRepository(dir.path());
    }
    ~Fixture() { delete repo; }
    QString path(const QString &name) const { return QDir(dir.path()).filePath(name); }
};

GitStatus statusOf(GitRepository &repo)
{
    GitStatus result;
    repo.status([&result](const GitResult &, const GitStatus &status) { result = status; });
    waitIdle(repo);
    return result;
}

GitResult runGit(GitRepository &repo, const QStringList &args)
{
    GitResult result;
    repo.run(args, [&result](const GitResult &r) { result = r; });
    waitIdle(repo);
    return result;
}

GitResult commitAll(GitRepository &repo, const QString &message)
{
    GitResult result;
    repo.commit(message, true, [&result](const GitResult &r) { result = r; });
    waitIdle(repo);
    return result;
}

int repositoryChecks()
{
    Fixture f;
    GitRepository &repo = *f.repo;
    bool isRepo = true;
    repo.detect([&isRepo](bool value) { isRepo = value; });
    waitIdle(repo);
    CHECK(40, !isRepo);
    CHECK(41, runGit(repo, {"init", "-q", "-b", "main"}).ok());
    repo.detect([&isRepo](bool value) { isRepo = value; });
    waitIdle(repo);
    CHECK(42, isRepo);

    GitStatus status = statusOf(repo);
    CHECK(43, status.initial && status.head == "main" && status.entries.isEmpty());

    writeFile(f.path("a.txt"), "uno\ndos\n");
    writeFile(f.path("dir/b.txt"), "b\n");
    status = statusOf(repo);
    CHECK(44, status.entries.size() == 2 && status.changedEntries().size() == 2 && status.stagedEntries().isEmpty());

    GitResult staged;
    repo.stage({"a.txt", "dir/b.txt"}, [&staged](const GitResult &r) { staged = r; });
    waitIdle(repo);
    CHECK(45, staged.ok());
    status = statusOf(repo);
    CHECK(46, status.stagedEntries().size() == 2 && status.stagedEntries()[0].index == 'A');

    GitResult unstaged;
    repo.unstage({"dir/b.txt"}, status.initial, [&unstaged](const GitResult &r) { unstaged = r; });  // before the first commit
    waitIdle(repo);
    CHECK(47, unstaged.ok());
    status = statusOf(repo);
    CHECK(48, status.stagedEntries().size() == 1 && status.changedEntries().size() == 1 && status.changedEntries()[0].untracked());

    // Without an identity the message must be actionable.
    qputenv("GIT_AUTHOR_NAME", "");
    qputenv("GIT_COMMITTER_NAME", "");
    qputenv("GIT_AUTHOR_EMAIL", "");
    qputenv("GIT_COMMITTER_EMAIL", "");
    GitResult noIdentity;
    repo.commit("sin identidad", false, [&noIdentity](const GitResult &r) { noIdentity = r; });
    waitIdle(repo);
    qputenv("GIT_AUTHOR_NAME", "Ana");
    qputenv("GIT_COMMITTER_NAME", "Ana");
    qputenv("GIT_AUTHOR_EMAIL", "ana@example.com");
    qputenv("GIT_COMMITTER_EMAIL", "ana@example.com");
    if (!noIdentity.ok()) CHECK(49, noIdentity.message().contains("user.name") || !noIdentity.message().isEmpty());

    GitResult committed;
    repo.commit("Primer commit\n\ncon cuerpo", false, [&committed](const GitResult &r) { committed = r; });
    waitIdle(repo);
    CHECK(50, committed.ok());
    status = statusOf(repo);
    CHECK(51, !status.initial && status.stagedEntries().isEmpty() && status.changedEntries().size() == 1);
    CHECK(52, commitAll(repo, "Segundo: todo").ok());  // stage all + commit
    CHECK(53, statusOf(repo).entries.isEmpty());

    // Modify, unstage, diff.
    writeFile(f.path("a.txt"), "uno\nDOS\ntres\n");
    GitDiff diff;
    repo.fileDiff("a.txt", false, [&diff](const GitResult &, const GitDiff &d) { diff = d; });
    waitIdle(repo);
    CHECK(54, diff.additions == 2 && diff.deletions == 1 && diff.rows.size() >= 3);
    repo.stage({"a.txt"}, nullptr);
    waitIdle(repo);
    repo.fileDiff("a.txt", true, [&diff](const GitResult &, const GitDiff &d) { diff = d; });
    waitIdle(repo);
    CHECK(55, diff.additions == 2);
    repo.unstage({"a.txt"}, false, nullptr);
    waitIdle(repo);
    status = statusOf(repo);
    CHECK(56, status.stagedEntries().isEmpty() && status.changedEntries().size() == 1);

    // Discard saves a recoverable copy first.
    QString backup;
    GitResult discarded;
    repo.discard(status.changedEntries(), false, [&](const GitResult &r, const QString &b) { discarded = r; backup = b; });
    waitIdle(repo);
    CHECK(57, discarded.ok() && !backup.isEmpty());
    QFile content(f.path("a.txt"));
    CHECK(88, content.open(QIODevice::ReadOnly));
    CHECK(58, content.readAll() == "uno\ndos\n");
    content.close();
    CHECK(59, QFileInfo::exists(QDir(backup).filePath("changes.patch")) && backup.contains(".git"));
    CHECK(60, runGit(repo, {"apply", QDir(backup).filePath("changes.patch")}).ok());  // the backup restores the lost edit
    QFile restored(f.path("a.txt"));
    CHECK(89, restored.open(QIODevice::ReadOnly));
    CHECK(61, restored.readAll() == "uno\nDOS\ntres\n");
    restored.close();
    CHECK(62, runGit(repo, {"checkout", "--", "a.txt"}).ok());

    // Untracked and staged-new files.
    writeFile(f.path("scratch.txt"), "borrador\n");
    writeFile(f.path("nuevo.txt"), "nuevo\n");
    repo.stage({"nuevo.txt"}, nullptr);
    waitIdle(repo);
    status = statusOf(repo);
    QList<GitEntry> toDiscard = status.entries;
    CHECK(63, toDiscard.size() == 2);
    repo.discard(toDiscard, false, [&](const GitResult &r, const QString &b) { discarded = r; backup = b; });
    waitIdle(repo);
    CHECK(64, discarded.ok() && !QFileInfo::exists(f.path("scratch.txt")) && !QFileInfo::exists(f.path("nuevo.txt")));
    CHECK(65, QFileInfo::exists(QDir(backup).filePath("untracked/scratch.txt")) && QFileInfo::exists(QDir(backup).filePath("changes.patch")));
    CHECK(66, statusOf(repo).entries.isEmpty());

    // Branches and a merge for the graph.
    GitResult created;
    repo.createBranch("feature", [&created](const GitResult &r) { created = r; });
    waitIdle(repo);
    CHECK(67, created.ok());
    writeFile(f.path("f.txt"), "feature\n");
    CHECK(68, commitAll(repo, "En la rama").ok());
    GitResult switched;
    repo.switchBranch("main", false, [&switched](const GitResult &r) { switched = r; });
    waitIdle(repo);
    CHECK(69, switched.ok());
    writeFile(f.path("m.txt"), "main\n");
    CHECK(70, commitAll(repo, "En main").ok());
    CHECK(71, runGit(repo, {"merge", "--no-ff", "-m", "Merge feature", "feature"}).ok());

    QList<GitBranch> branches;
    repo.branches([&branches](const GitResult &, const QList<GitBranch> &b) { branches = b; });
    waitIdle(repo);
    CHECK(72, branches.size() == 2);
    int current = 0;
    for (const GitBranch &b : branches) current += b.current ? 1 : 0;
    CHECK(73, current == 1);

    QList<GitCommit> commits;
    repo.log(0, 50, [&commits](const GitResult &, const QList<GitCommit> &c) { commits = c; });
    waitIdle(repo);
    CHECK(74, commits.size() == 5 && commits[0].subject == "Merge feature" && commits[0].parents.size() == 2);
    CHECK(75, commits[0].refs.join(",").contains("HEAD -> main"));
    QVector<QString> lanes;
    assignGraph(commits, lanes);
    CHECK(76, commits[0].graph.parentLanes.size() == 2 && commits[0].graph.parentLanes[0] != commits[0].graph.parentLanes[1]);
    CHECK(77, lanes.isEmpty());  // every line closes at the root commit

    QList<GitFileChange> files;
    repo.commitFiles(commits[0].hash, [&files](const GitResult &, const QList<GitFileChange> &c) { files = c; });
    waitIdle(repo);
    CHECK(78, files.size() == 1 && files[0].path == "f.txt" && files[0].status == "A");  // a merge shows what it brought in
    GitDiff commitDiff;
    repo.commitDiff(commits[1].hash, "m.txt", [&commitDiff](const GitResult &, const GitDiff &d) { commitDiff = d; });
    waitIdle(repo);
    CHECK(79, commitDiff.additions == 1);
    GitDiff mergeDiff;
    repo.commitDiff(commits[0].hash, "f.txt", [&mergeDiff](const GitResult &, const GitDiff &d) { mergeDiff = d; });
    waitIdle(repo);
    CHECK(87, mergeDiff.additions == 1);

    // Deleting: safe delete refuses an unmerged branch, force deletes it.
    CHECK(80, runGit(repo, {"switch", "-c", "wip"}).ok());
    writeFile(f.path("w.txt"), "wip\n");
    CHECK(81, commitAll(repo, "wip").ok());
    CHECK(82, runGit(repo, {"switch", "main"}).ok());
    GitResult deleted;
    repo.deleteBranch("wip", false, [&deleted](const GitResult &r) { deleted = r; });
    waitIdle(repo);
    CHECK(83, !deleted.ok() && !deleted.message().isEmpty());
    repo.deleteBranch("wip", true, [&deleted](const GitResult &r) { deleted = r; });
    waitIdle(repo);
    CHECK(84, deleted.ok());

    // Destroying the repository with a command in flight must not call back into the (destroyed) owner.
    {
        bool called = false;
        auto *doomed = new GitRepository(f.dir.path());
        doomed->run({"log", "-n", "1"}, [&called](const GitResult &) { called = true; }, {}, true);
        doomed->run({"status"}, [&called](const GitResult &) { called = true; }, {}, true);
        delete doomed;
        QElapsedTimer settle;
        settle.start();
        while (settle.elapsed() < 300) QCoreApplication::processEvents(QEventLoop::AllEvents, 20);
        CHECK(90, !called);
    }

    // Errors are reported, not thrown.
    GitResult bad;
    repo.switchBranch("no-existe", false, [&bad](const GitResult &r) { bad = r; });
    waitIdle(repo);
    CHECK(85, !bad.ok() && !bad.message().isEmpty());

    // Commands are serialized: queued jobs all run, in order.
    QStringList order;
    for (int i = 0; i < 5; ++i) repo.run({"rev-parse", "HEAD"}, [&order, i](const GitResult &) { order << QString::number(i); }, {}, true);
    waitIdle(repo);
    CHECK(86, order == QStringList({"0", "1", "2", "3", "4"}));
    return 0;
}


bool gitSync(const QString &cwd, const QStringList &args)
{
    QProcess process;
    process.setWorkingDirectory(cwd);
    process.start("git", args);
    return process.waitForFinished(20000) && process.exitCode() == 0;
}

GitResult callGit(GitRepository &repo, const std::function<void(GitRepository::Callback)> &operation)
{
    GitResult result;
    operation([&result](const GitResult &r) { result = r; });
    waitIdle(repo);
    return result;
}

QByteArray readText(const QString &path)
{
    QFile file(path);
    return file.open(QIODevice::ReadOnly) ? file.readAll() : QByteArray();
}

int remoteChecks()
{
    Fixture f;
    const QString base = f.dir.path();
    const QString a = base + "/a";
    const QString b = base + "/b";
    const QString c = base + "/c";
    CHECK(200, gitSync(base, {"init", "-q", "--bare", "-b", "main", "origin.git"}));
    CHECK(201, gitSync(base, {"init", "-q", "-b", "main", "a"}));
    CHECK(202, gitSync(a, {"remote", "add", "origin", base + "/origin.git"}));
    GitRepository ra(a);
    writeFile(a + "/one.txt", "1\n");
    CHECK(203, commitAll(ra, "uno").ok());

    // A branch with no upstream: the problem is recognized, then publishing works.
    GitResult res = callGit(ra, [&](auto done) { ra.push("origin", "main", false, done); });
    CHECK(204, !res.ok() && res.problem() == GitProblem::NoUpstream);
    res = callGit(ra, [&](auto done) { ra.push("origin", "main", true, done); });
    CHECK(205, res.ok());
    GitStatus status = statusOf(ra);
    CHECK(206, status.upstream == "origin/main" && status.ahead == 0 && status.behind == 0);
    QStringList remotes;
    ra.remotes([&remotes](const GitResult &, const QStringList &names) { remotes = names; });
    waitIdle(ra);
    CHECK(207, remotes == QStringList({"origin"}));

    // Another clone receives what A sends.
    CHECK(210, gitSync(base, {"clone", "-q", "origin.git", "b"}));
    GitRepository rb(b);
    writeFile(a + "/two.txt", "2\n");
    CHECK(211, commitAll(ra, "dos").ok());
    writeFile(a + "/three.txt", "3\n");
    CHECK(212, commitAll(ra, "tres").ok());
    CHECK(213, statusOf(ra).ahead == 2);
    CHECK(214, callGit(ra, [&](auto done) { ra.push("", "", false, done); }).ok());
    CHECK(215, statusOf(ra).ahead == 0);
    CHECK(216, callGit(rb, [&](auto done) { rb.fetch(done); }).ok());
    CHECK(217, statusOf(rb).behind == 2 && statusOf(rb).ahead == 0);
    CHECK(218, callGit(rb, [&](auto done) { rb.pull(GitPullMode::FastForwardOnly, done); }).ok());
    CHECK(219, statusOf(rb).behind == 0 && QFileInfo::exists(b + "/three.txt"));

    // Diverged: fast-forward is refused (and explained), merge reconciles, then the push goes through.
    writeFile(a + "/a2.txt", "a\n");
    CHECK(220, commitAll(ra, "desde A").ok());
    CHECK(221, callGit(ra, [&](auto done) { ra.push("", "", false, done); }).ok());
    writeFile(b + "/b2.txt", "b\n");
    CHECK(222, commitAll(rb, "desde B").ok());
    CHECK(223, callGit(rb, [&](auto done) { rb.fetch(done); }).ok());
    status = statusOf(rb);
    CHECK(224, status.ahead == 1 && status.behind == 1);
    res = callGit(rb, [&](auto done) { rb.pull(GitPullMode::FastForwardOnly, done); });
    CHECK(225, !res.ok() && res.problem() == GitProblem::Diverged);
    CHECK(226, callGit(rb, [&](auto done) { rb.pull(GitPullMode::Merge, done); }).ok());
    status = statusOf(rb);
    CHECK(227, status.behind == 0 && status.ahead == 2 && QFileInfo::exists(b + "/a2.txt"));
    CHECK(228, callGit(rb, [&](auto done) { rb.push("", "", false, done); }).ok());
    CHECK(229, callGit(ra, [&](auto done) { ra.pull(GitPullMode::FastForwardOnly, done); }).ok());

    // A rejected push is never forced.
    writeFile(a + "/a3.txt", "a3\n");
    CHECK(230, commitAll(ra, "a3").ok());
    CHECK(231, callGit(ra, [&](auto done) { ra.push("", "", false, done); }).ok());
    writeFile(b + "/b3.txt", "b3\n");
    CHECK(232, commitAll(rb, "b3").ok());
    res = callGit(rb, [&](auto done) { rb.push("", "", false, done); });
    CHECK(233, !res.ok() && res.problem() == GitProblem::Rejected && res.message().contains("Recibir"));
    CHECK(234, callGit(rb, [&](auto done) { rb.pull(GitPullMode::Merge, done); }).ok());
    CHECK(235, callGit(rb, [&](auto done) { rb.push("", "", false, done); }).ok());
    CHECK(236, callGit(ra, [&](auto done) { ra.pull(GitPullMode::FastForwardOnly, done); }).ok());

    // A merge conflict can be abandoned.
    writeFile(a + "/one.txt", "desde A\n");
    CHECK(240, commitAll(ra, "A edita").ok());
    CHECK(241, callGit(ra, [&](auto done) { ra.push("", "", false, done); }).ok());
    writeFile(b + "/one.txt", "desde B\n");
    CHECK(242, commitAll(rb, "B edita").ok());
    res = callGit(rb, [&](auto done) { rb.pull(GitPullMode::Merge, done); });
    CHECK(243, !res.ok());
    status = statusOf(rb);
    CHECK(244, status.conflicts().size() == 1 && status.conflicts()[0].path == "one.txt");
    bool inProgress = false;
    rb.mergeInProgress([&inProgress](bool value) { inProgress = value; });
    waitIdle(rb);
    CHECK(245, inProgress);
    CHECK(246, callGit(rb, [&](auto done) { rb.abortMerge(done); }).ok());
    rb.mergeInProgress([&inProgress](bool value) { inProgress = value; });
    waitIdle(rb);
    CHECK(247, !inProgress && statusOf(rb).conflicts().isEmpty() && readText(b + "/one.txt") == "desde B\n");

    // No remote: the problem is explained, and nothing breaks.
    CHECK(250, gitSync(base, {"init", "-q", "-b", "main", "c"}));
    GitRepository rc(c);
    writeFile(c + "/x.txt", "x\n");
    CHECK(251, commitAll(rc, "x").ok());
    res = callGit(rc, [&](auto done) { rc.push("", "", false, done); });
    CHECK(252, !res.ok() && res.problem() == GitProblem::NoRemote && res.message().contains("remoto"));
    QStringList none{"x"};
    rc.remotes([&none](const GitResult &, const QStringList &names) { none = names; });
    waitIdle(rc);
    CHECK(253, none.isEmpty());

#ifndef Q_OS_WIN
    // A slow download can be cancelled (git runs a command that sleeps, standing in for an unresponsive server).
    {
        GitResult slow;
        bool finished = false;
        QElapsedTimer clock;
        clock.start();
        rc.run({"-c", "protocol.ext.allow=always", "fetch", "ext::sleep 30"}, [&](const GitResult &r) { slow = r; finished = true; }, {}, false,
               8 * 1024 * 1024, 60000);
        QElapsedTimer wait;
        wait.start();
        while (wait.elapsed() < 500) QCoreApplication::processEvents(QEventLoop::AllEvents, 20);
        CHECK(260, rc.isBusy());
        rc.cancelCurrent();
        waitIdle(rc);
        CHECK(261, finished && slow.cancelled && !slow.ok() && slow.problem() == GitProblem::Cancelled && clock.elapsed() < 8000);
        CHECK(262, callGit(rc, [&](auto done) { rc.run({"status"}, done, {}, true); }).ok());  // the queue keeps working
    }
#endif

    // Stash: set work aside, bring it back, drop it.
    writeFile(a + "/one.txt", "cambio a medias\n");
    writeFile(a + "/borrador.txt", "nuevo sin seguimiento\n");
    CHECK(270, statusOf(ra).entries.size() == 2);
    CHECK(271, callGit(ra, [&](auto done) { ra.stashPush("it's: un \"apartado\" con símbolos", true, done); }).ok());
    CHECK(272, statusOf(ra).entries.isEmpty() && !QFileInfo::exists(a + "/borrador.txt"));
    QList<GitStash> stashes;
    ra.stashList([&stashes](const GitResult &, const QList<GitStash> &list) { stashes = list; });
    waitIdle(ra);
    CHECK(273, stashes.size() == 1 && stashes[0].index == 0 && stashes[0].message.contains("símbolos"));
    CHECK(274, callGit(ra, [&](auto done) { ra.stashApply(0, done); }).ok());
    CHECK(275, statusOf(ra).entries.size() == 2);
    ra.stashList([&stashes](const GitResult &, const QList<GitStash> &list) { stashes = list; });
    waitIdle(ra);
    CHECK(276, stashes.size() == 1);  // apply keeps it
    CHECK(277, runGit(ra, {"checkout", "--", "one.txt"}).ok() && QFile::remove(a + "/borrador.txt") && statusOf(ra).entries.isEmpty());
    CHECK(278, callGit(ra, [&](auto done) { ra.stashPop(0, done); }).ok());
    ra.stashList([&stashes](const GitResult &, const QList<GitStash> &list) { stashes = list; });
    waitIdle(ra);
    CHECK(279, stashes.isEmpty() && statusOf(ra).entries.size() == 2);  // pop applies and removes
    CHECK(280, callGit(ra, [&](auto done) { ra.stashPush("otro", true, done); }).ok());
    res = callGit(ra, [&](auto done) { ra.stashDrop(0, done); });
    CHECK(281, res.ok() && QString::fromUtf8(res.out).contains("Dropped"));  // the output carries the hash that allows recovery
    ra.stashList([&stashes](const GitResult &, const QList<GitStash> &list) { stashes = list; });
    waitIdle(ra);
    CHECK(282, stashes.isEmpty());
    return 0;
}

} // namespace

int main(int argc, char **argv)
{
    QCoreApplication application(argc, argv);
    if (int code = parserChecks()) return code;
    if (int code = repositoryChecks()) return code;
    return remoteChecks();
}
