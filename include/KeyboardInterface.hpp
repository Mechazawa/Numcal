#pragma once

class KeyboardInterface {
    public:
        virtual void onPress(char row, char column) {};
        virtual void onRelease(char row, char column) {};
        virtual void onLongPress(char row, char column) {};
};
