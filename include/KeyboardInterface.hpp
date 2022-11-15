#include <U8g2lib.h>

#pragma once

class KeyboardInterface {
    public:
        virtual void onPress(char row, char column) {};
        virtual void onRelease(char row, char column) {};
        virtual void onLongPress(char row, char column) {};
        virtual void draw(U8G2* u8g2) {};
        virtual void tick(const unsigned long ms) {};
        virtual void onShow() {};
        virtual void onHide() {};
};
