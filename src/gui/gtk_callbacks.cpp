#include "gtk_callbacks.h"
#include <iostream>
#include "MainPickerWindow.h"
#include "../MySteam.h"
#include "../globals.h"

// See comments in the header file

extern "C" 
{

    void 
    on_close_button_clicked() {
        gtk_main_quit();
        gtk_widget_destroy(g_main_gui->get_main_window());

        delete g_main_gui;
        g_main_gui = NULL;

        g_steam->quit_game();
    }
    // => on_close_button_clicked

    void
    on_store_button_clicked() {
        std::cerr << "Saving stats and achievements." << std::endl;
        const std::map<std::string, bool> pending_achs = g_steam->get_pending_ach_modifications();
        const std::map<std::string, double> pending_stats = g_steam->get_pending_stat_modifications();
        
        //TODO:
        //
        //commit changes
        //MySteam::commit_changes() //reset pending changes too?
        //
        // pull out the same game achievement population code from on_game_row_activated
        // g_main_gui->reset_achievements_list();
        // g_main_gui->confirm_stats_list();

    }
    // => on_store_button_clicked

    void 
    on_ask_game_refresh() {
        g_main_gui->reset_game_list();
        g_steam->refresh_owned_apps();

        for(Game_t app : g_steam->get_all_games_with_stats()) {
            g_main_gui->add_to_game_list(app);
        }

        g_steam->refresh_icons();
        g_main_gui->confirm_game_list();
    }
    // => on_ask_game_refresh


    void 
    on_main_window_show() {
        on_ask_game_refresh(); //Run this async?
    }
    // => on_main_window_show


    void
    on_search_changed(GtkWidget* search_widget) {
        const char* filter_text = gtk_entry_get_text( GTK_ENTRY(search_widget) );

        //if !g_steam->isgamerunning
        g_main_gui->filter_games(filter_text);
        //else 
        //g_main_gui->filter_stats(filter_text)
    }
    // => on_search_changed


    void 
    on_game_row_activated(GtkListBox *box, GtkListBoxRow *row) {

        const AppId_t appId = g_main_gui->get_corresponding_appid_for_row(row);

        if( appId != 0 ) {
            g_main_gui->switch_to_stats_page();
            g_steam->launch_game(appId);
            // Get_achievements from game server
            std::vector<Achievement_t> achievements = g_steam->get_achievements();

            g_main_gui->reset_achievements_list();

            //TODO: just pass in the array directly?
            for(Achievement_t achievement : achievements) {
                g_main_gui->add_to_achievement_list(achievement);
            }

            g_main_gui->confirm_stats_list();

        } else {
            std::cerr << "An error occurred figuring out which app to launch.. You can report this to the developer." << std::endl;
        }

    }
    // => on_game_row_activated

    void
    on_back_button_clicked() {
        g_steam->quit_game();
        g_main_gui->switch_to_games_page();
    }
    // => on_back_button_clicked
}