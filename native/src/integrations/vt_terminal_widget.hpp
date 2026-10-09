#pragma once

#include <QWidget>
#include <QByteArray>
#include <QString>
#include <memory>
#include <vterm.h>

class VtTerminalWidget final : public QWidget {
    Q_OBJECT
public:
    explicit VtTerminalWidget(QWidget *parent = nullptr);
    ~VtTerminalWidget() override;
    void feed(const QByteArray &bytes);
    void pasteText(const QString &text);

signals:
    void inputReady(const QByteArray &bytes);
    void terminalResized(int columns, int rows);

protected:
    bool event(QEvent *event) override;
    void paintEvent(QPaintEvent *event) override;
    void resizeEvent(QResizeEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;

private:
    static int damage(VTermRect rect, void *user);
    static int cursorMoved(VTermPos pos, VTermPos oldPos, int visible, void *user);
    static int terminalProperty(VTermProp property, VTermValue *value, void *user);
    static int scrollbackPush(int columns, const VTermScreenCell *cells, void *user);
    static int scrollbackPop(int columns, VTermScreenCell *cells, void *user);
    static int scrollbackClear(void *user);
    VTermModifier modifiers(Qt::KeyboardModifiers modifiers) const;
    void scrollbackBy(int lines);
    void sendMouseButton(int button, bool pressed, const QPointF &position,
        Qt::KeyboardModifiers modifiers);
    void flushInput();
    struct State;
    std::unique_ptr<State> m_state;
};
