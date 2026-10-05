#pragma once

#include <QWidget>
#include <QByteArray>
#include <memory>
#include <vterm.h>

class VtTerminalWidget final : public QWidget {
    Q_OBJECT
public:
    explicit VtTerminalWidget(QWidget *parent = nullptr);
    ~VtTerminalWidget() override;
    void feed(const QByteArray &bytes);

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
    VTermModifier modifiers(Qt::KeyboardModifiers modifiers) const;
    void sendMouseButton(int button, bool pressed, const QPointF &position,
        Qt::KeyboardModifiers modifiers);
    void flushInput();
    struct State;
    std::unique_ptr<State> m_state;
};
