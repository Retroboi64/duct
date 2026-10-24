/**
 * Project32 Duct - C Language Integration Example
 * Demonstrates consuming duct.dll from standard C.
 */

#include <stdio.h>
#include "../include/duct.h"

int main(int argc, char* argv[]) {
    printf("[C Example] Project32 Duct Graphics Subsystem C-ABI Test\n");

    /* Create dummy window handle descriptor for test */
    DuctWindowHandle win;
    win.native_window = (void*)0x12345678;
    win.native_display = NULL;

    DuctInitConfig cfg;
    cfg.window = win;
    cfg.width = 1280;
    cfg.height = 720;
    cfg.vsync = true;
    cfg.debug_mode = true;
    cfg.app_name = "C Integration Sample";

    DuctHandle duct = NULL;
    DuctResult res = duct_create(&cfg, &duct);

    if (res != DUCT_SUCCESS) {
        printf("[C Example] duct_create returned code: %d\n", res);
        return 1;
    }

    printf("[C Example] Backend Name: %s\n", duct_get_backend_name(duct));

    DuctColor cornflower = { 0.392f, 0.584f, 0.929f, 1.0f };
    duct_begin_frame(duct, cornflower);
    duct_draw(duct, 3, 0);
    duct_end_frame(duct);
    duct_present(duct);

    duct_destroy(duct);
    printf("[C Example] Test finished successfully.\n");
    return 0;
}
