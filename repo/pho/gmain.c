/* -*- Mode: C; tab-width: 4; indent-tabs-mode: nil; c-basic-offset: 4 -*- */
/*
 * gmain.c: gtk main routines for pho, an image viewer.
 *
 * Copyright 2004 by Akkana Peck.
 * You are free to use or modify this code under the Gnu Public License.
 */

#include "pho.h"
#include "dialogs.h"
#include "exif/phoexif.h"

#include <gdk/gdk.h>
#include <gdk/gdkkeysyms.h>

#include <stdlib.h>       /* for getenv() */
#include <unistd.h>
#include <string.h>
#include <ctype.h>

char * gCapFileFormat = "Captions";

#if 0
GDK_space			0x020
GDK_exclam			0x021
GDK_quotedbl		0x022
GDK_numbersign		0x023
GDK_dollar			0x024
GDK_percent			0x025
GDK_ampersand		0x026
GDK_apostrophe		0x027
GDK_quoteright		0x027
GDK_parenleft		0x028
GDK_parenright		0x029
GDK_asterisk		0x02a
GDK_plus			0x02b
GDK_comma			0x02c
GDK_minus			0x02d
GDK_period			0x02e
GDK_slash			0x02f
GDK_colon			0x03a
GDK_semicolon		0x03b
GDK_less			0x03c
GDK_equal			0x03d
GDK_greater			0x03e
GDK_question		0x03f
GDK_at				0x040
GDK_bracketleft		0x05b
GDK_backslash		0x05c
GDK_bracketright	0x05d
GDK_asciicircum		0x05e
GDK_underscore		0x05f
GDK_grave			0x060
GDK_quoteleft		0x060
GDK_braceleft		0x07b
GDK_bar				0x07c
GDK_braceright		0x07d
GDK_asciitilde		0x07e
#endif

/* Toggle a variable between two modes, preferring the first.
 * If it's anything but mode1 it will end up as mode1.
 */
#define ToggleBetween(val,mode1,mode2) (val != mode1 ? mode1 : mode2)

/* If we're switching into scaling mode because the user pressed - or +/=,
 * which scaling mode should we switch into?
 */
static int ModeForScaling(int oldmode)
{
    switch (oldmode)
    {
      case PHO_SCALE_IMG_RATIO:
      case PHO_SCALE_FULLSIZE:
      case PHO_SCALE_NORMAL:
        return PHO_SCALE_IMG_RATIO;

      case PHO_SCALE_FIXED:
          return PHO_SCALE_FIXED;

      case PHO_SCALE_FULLSCREEN:
      default:
        return PHO_SCALE_SCREEN_RATIO;
    }
}

//add by sim1
int gRepeat = 0;
int IfRepeat()
{
    return gRepeat;
}

#if 0
static void RunPhoCommand()
{
    int i;
    char* cmd = getenv("PHO_CMD");
    if (cmd == 0) cmd = "gimp";
    else if (! *cmd) {
        if (gDebug)
            printf("PHO_CMD set to NULL, not running anything\n");
        return;
    }
    else if (gDebug)
        printf("Calling PHO_CMD %s\n", cmd);

    gint gargc;
    gchar** gargv;
    gchar** new_argv = 0;
    GError *gerror = 0;
    if (! g_shell_parse_argv (cmd, &gargc, &gargv, &gerror)) {
        fprintf(stderr, "Couldn't parse PHO_CMD %s\nError was %s",
                cmd, gerror->message);
        return;
    }

    /* PHO_CMD command can have a %s in it; if it does, substitute filename. */
    int added_arg = 0;
    for (i=1; i<gargc; ++i) {
        if (gargv[i][0] == '%' && gargv[i][1] == 's') {
            gargv[i] = gCurImage->filename;
            added_arg = 1;
        }
    }
    /* If it didn't have a %s, then we have to allocate a whole new argv array
     * so we can add the filename argument at the end.
     * Note that glib expects the argv to be zero terminated --
     * it provides an argc from g_shell_parse_args() but doesn't
     * take one in g_spawn_async().
     */
    if (! added_arg) {
        int new_argc = gargc + 2;
        gchar** new_argv = malloc(sizeof(gchar *) * new_argc);
        for (i=0; i<gargc; ++i)
            new_argv[i] = gargv[i];
        new_argv[gargc] = gCurImage->filename;
        new_argv[gargc+1] = 0;
        gargv = new_argv;
    }

    if (! g_spawn_async(NULL, gargv,
                        NULL, G_SPAWN_SEARCH_PATH, NULL, NULL, NULL, &gerror))
        fprintf(stderr, "Couldn't spawn %s: \"%s\"\n", cmd, gerror->message);

    if (new_argv)
        free(new_argv);
}
#endif

void TryScale(float times)
{
    /* Save the view modes, in case this fails */
    int saveScaleMode = gScaleMode;
    double saveScaleRatio = gScaleRatio;
    int saveDisplayMode = gDisplayMode;

    if (SetViewModes(gDisplayMode, ModeForScaling(gScaleMode),
                     gScaleRatio * times) == 0)
        return;

    /* Oops! It didn't work. Try to reset back to previous settings */
    SetViewModes(saveDisplayMode, saveScaleMode, saveScaleRatio);
}

//mod by sim1
gint HandleGlobalKeys(GtkWidget* widget, GdkEventKey* event)
{
    if (gDebug)
        printf("00000 key: %lu\n", (unsigned long)(event->keyval));

    #if 0
    if (event->state) {
        switch (event->keyval)
        {
            case GDK_f:
                /* Don't respond to ctrl-F -- that might be an attempt
                 * to edit in a text field in the keywords dialog.
                 * But we don't do anything on f with any modifier key either.
                 */
                return FALSE;
            case GDK_0:
            case GDK_1:
            case GDK_2:
            case GDK_3:
            case GDK_4:
            case GDK_5:
            case GDK_6:
            case GDK_7:
            case GDK_8:
            case GDK_9:
                if (event->state & GDK_MOD1_MASK) { /* alt-num: add 10 to num */
                    ToggleNoteFlag(gCurImage, event->keyval - GDK_0 + 10);
                    return TRUE;
                }
                return FALSE;
            case GDK_equal:
                if (event->state & GDK_CONTROL_MASK) {
                    TryScale(1.25);
                    return TRUE;
                }
                return FALSE;
            case GDK_KP_Subtract:
                if (event->state & GDK_CONTROL_MASK) {
                    TryScale(.8);
                    return TRUE;
                }
                return FALSE;
            default:
                printf("11111 key: %lu\n", (unsigned long)(event->keyval));
                return FALSE;
        }
    }
    #endif

    /* Now we know no modifier keys were down. */
    switch (event->keyval)
    {
      case GDK_D:
          DeleteImage(gCurImage);
          break;
      case GDK_n:
      case GDK_space:
          if (NextImage() != 0) {
              #if 0
              if (Prompt("Quit pho?", "Quit", "Continue", "qx \n", "cn") != 0)
                  EndSession();
              #endif
          }
          return TRUE;
      case GDK_p:
      case GDK_BackSpace:
      case GDK_Page_Up:
      case GDK_KP_Page_Up:
          PrevImage();
          return TRUE;
      case GDK_P:
      case GDK_S:
          //slideshow
          gDelayMillis = 1000;
          gRepeat=1;
          ShowImage();
          return TRUE;
      case GDK_Home:
      case GDK_0:
      case GDK_asciicircum:
          gCurImage = 0;
          NextImage();
          return TRUE;
      case GDK_End:
      case GDK_G:
      case GDK_dollar:
          gCurImage = gFirstImage->prev;
          ThisImage();
          return TRUE;
      case GDK_x:
      case GDK_F:
          SetViewModes(gDisplayMode,
                       ToggleBetween(gScaleMode, PHO_SCALE_FULLSCREEN, PHO_SCALE_NORMAL),
                       1.);
          return TRUE;
      case GDK_f:
          SetViewModes((gDisplayMode == PHO_DISPLAY_PRESENTATION)
                       ? PHO_DISPLAY_NORMAL
                       : PHO_DISPLAY_PRESENTATION,
                       gScaleMode, gScaleRatio);
          return TRUE;
      case GDK_r:
      case GDK_Right:
      case GDK_KP_Right:
          ScaleAndRotate(gCurImage, 90);
          return TRUE;
      case GDK_R:
      case GDK_Left:
      case GDK_KP_Left:
          ScaleAndRotate(gCurImage, 270);
          return TRUE;
      case GDK_i:
      case GDK_plus:
      case GDK_KP_Add:
          if (event->state & GDK_CONTROL_MASK) {
              TryScale(1.5);
          } else {
              TryScale(1.25);
          }
          return TRUE;
      case GDK_o:
      case GDK_minus:
      case GDK_KP_Subtract:
          if (event->state & GDK_CONTROL_MASK) {
              TryScale(.6);
          } else {
              TryScale(.8);
          }
          return TRUE;
      case GDK_equal:
      case GDK_period:
          SetViewModes(gDisplayMode, PHO_SCALE_NORMAL, 1.);
          return TRUE;
      case GDK_Up:
      case GDK_Down:
          ScaleAndRotate(gCurImage, 180);
          return TRUE;
      #if 0
      case GDK_X:  /* start gimp, or some other app */
          RunPhoCommand();
          return TRUE;
      case GDK_I:
          ToggleInfo();
          return TRUE;
      case GDK_K:
          ToggleKeywordsMode();
          return TRUE;
      case GDK_O:
          ChangeWorkingFileSet();
          return TRUE;
      case GDK_0:
      case GDK_1:
      case GDK_2:
      case GDK_3:
      case GDK_4:
      case GDK_5:
      case GDK_6:
      case GDK_7:
      case GDK_8:
      case GDK_9:
          ToggleNoteFlag(gCurImage, event->keyval - GDK_0);
          return TRUE;
      #endif
      case GDK_Escape:
          /* If we're in slideshow mode, cancel the slideshow */
          if (gDelayMillis > 0) {
              gDelayMillis = 0;
          }
          return TRUE;
      case GDK_q:
          EndSession();
          return TRUE;
      case GDK_c:
          if (event->state & GDK_CONTROL_MASK) {
              EndSession();
              return TRUE;
          }
      default:
          if (gDebug)
              printf("222222 Unknown key: %lu\n", (unsigned long)(event->keyval));
          return FALSE;
    }
    /* Keep gcc 2.95 happy: */
    return FALSE;
}

PhoImage* AddImage(char* filename)
{
    PhoImage* img = NewPhoImage(filename);
    if (gDebug)
        printf("Adding image %s\n", filename);
    if (!img) {
        fprintf(stderr, "Out of memory!\n");
        exit(1);
    }
    /* Make img the new last image in the list */
    AppendItem(img);
    return img;
}

static void parseGeom(char* geom, int* width, int* height)
{
    int num;
    if (!isdigit(*geom)) {
        printf("Geometry '%s' must start with a digit\n", geom);
        Usage();
    }
    num = atoi(geom);
    if (num > 0)
        *width = num;

    /* Now see if there's a height */
    while (*(++geom)) {
        if (*geom == 'x') {
            ++geom;
            if (!isdigit(*geom)) {
                printf("Number after 'x' in geometry must start with a digit, not %s\n", geom);
                Usage();
            }
            num = atoi(geom);
            if (num > 0)
                *height = num;
            return;
        }
    }
}

/* CheckArg takes a string, like -Pvg, and sets all the relevant flags. */
static void CheckArg(char* arg)
{
    for ( ; *arg != 0; ++arg)
    {
        if (*arg == '-')
            ;
        else if (*arg == 'd')
            gDebug = 1;
        else if (*arg == 'h')
            Usage();
        else if (*arg == 'v')
            VerboseHelp();
        else if (*arg == 'n')
            gMakeNewWindows = 1;
        else if (*arg == 'p') {
            gDisplayMode = PHO_DISPLAY_PRESENTATION;
            if (arg[1] == '\0') {
                /* Normal presentation mode -- center everything
                 * and size to the current display.
                 */
                gPresentationWidth = 0;
                gPresentationHeight = 0;
            } else {
                char* geom = arg+1;
                if (*geom == 'p') ++geom;
                gPresentationWidth = 0;
                gPresentationHeight = 0;
                parseGeom(geom, &gPresentationWidth, &gPresentationHeight);

                /* Check for width without height, in case the user
                 * ran something like pho -p1024, which is almost
                 * certainly a mistake.
                 */
                if (gPresentationWidth && !gPresentationHeight) {
                    fprintf(stderr,
                            "Presentation width %d given with no height.\n\n",
                            gPresentationWidth);
                    Usage();
                }
                /* Note there's no check for the opposite, height without width.
                 * For that, the user would have had to specify something like
                 * 0x768; in the unlikely event the user does that,
                 * maybe it means they really want it for some strange reason.
                 */
            }
        } else if (*arg == 'P')
            gDisplayMode = PHO_DISPLAY_NORMAL;
        else if (*arg == 'k') {
            gDisplayMode = PHO_DISPLAY_KEYWORDS;
            gScaleMode = PHO_SCALE_FIXED;
            gScaleRatio = 0.0;
        } else if (*arg == 's') {
            /* find the slideshow delay time, from e.g. pho -s2 */
            if (isdigit(arg[1]) || arg[1] == '.')
                gDelayMillis = (int)(atof(arg+1) * 1000.);
            else Usage();
            if (gDebug)
                printf("Slideshow delay %d milliseconds\n", gDelayMillis);
        } else if (*arg == 'r') {
            gRepeat = 1;
        } else if (*arg == 'c') {
            gCapFileFormat = strdup(arg+1);
            if (gDebug)
                printf("Format set to '%s'\n", gCapFileFormat);

            /* Can't follow this with any other letters -- they're all
             * part of the filename -- so return.
             */
            return;
        }
    }
}

int main(int argc, char** argv)
{
    /* Initialize some defaults from environment variables,
     * before reading cmdline args.
     */
    int options = 1;

    char* env = getenv("PHO_ARGS");
    if (env && *env)
        CheckArg(env);

    while (argc > 1)
    {
        if (argv[1][0] == '-' && options) {
            if (strcmp(argv[1], "--"))
                CheckArg(argv[1]);
            else
                options = 0;
        }
        else {
            AddImage(argv[1]);
        }
        --argc;
        ++argv;
    }

    if (gFirstImage == 0)
        Usage();

    /* Initialize some variables associated with the notes flags */
    InitNotes();

    /* See http://www.gtk.org/tutorial */
    gtk_init(&argc, &argv);

    /* Must init rgb system explicitly, else we'll crash
     * in the first gdk_pixbuf_render_to_drawable(),
     * calling gdk_draw_rgb_image_dithalign():
     * (Is this still true in gtk2?)
     */
    gdk_rgb_init();

    gPhysMonitorWidth = gMonitorWidth = gdk_screen_width();
    gPhysMonitorHeight = gMonitorHeight = gdk_screen_height();

    /* Load the first image */
    if (NextImage() != 0)
        exit(1);

    gtk_main();
    return 0;
}

