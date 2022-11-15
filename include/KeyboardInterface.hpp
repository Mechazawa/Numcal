#include <U8g2lib.h>

#pragma once

class KeyboardInterface {
    protected:
        bool drawNext = false;

    public:
        virtual void onPress(char row, char column) {};
        virtual void onRelease(char row, char column) {};
        virtual void onLongPress(char row, char column) {};
        virtual void draw(U8G2* u8g2) {};

        void forceDraw(U8G2* u8g2) {
            this->drawNext = true;
            this->draw(u8g2);
        }
};
