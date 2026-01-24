/*============================================================================
  epub2txt v2
  epub2txt.c
  Copyright (c)2020-2024 Kevin Boone, GPL v3.0
============================================================================*/

#define _GNU_SOURCE 1
#define _POSIX_C_SOURCE 200809L // For strdup, realpath, mkdtemp, and other POSIX functions

#include <stdio.h>
#include <stdlib.h> // For mkdtemp, realpath, malloc, free, getenv
#include <string.h> // For strdup, strcmp, strerror, strrchr, strndup
#include <unistd.h> // For access, getpid
#include <sys/types.h>
#include <sys/stat.h>
#include <errno.h>
#include <stdarg.h> // Required for asprintf prototype and its usage
#include <limits.h> // For PATH_MAX

#ifndef __APPLE__
#include <malloc.h>
#endif

// Explicitly declare asprintf if the system headers aren't providing it
#if defined(__GNUC__) && !defined(asprintf)
extern int asprintf(char **strp, const char *fmt, ...)
    __attribute__((__format__(__printf__, 2, 3)));
#elif !defined(asprintf)
extern int asprintf(char **strp, const char *fmt, ...);
#endif

// Explicitly declare mkdtemp if not found via stdlib.h with feature test macros
// This is unusual on macOS but will satisfy the "undeclared function" error.
#ifndef mkdtemp // Check if it's a macro first (unlikely for mkdtemp)
extern char *mkdtemp(char *template);
#endif

#include "epub2txt.h"
#include "log.h"
#include "list.h"

#include "custom_string.h"
#include "sxmlc.h"
#include "xhtml.h"
#include "util.h"

// APPNAME is defined by the Makefile compiler arguments, e.g., -DAPPNAME=\"epub2txt\"

static char *tempdir = NULL;

/*============================================================================
  epub2txt_unescape_html
============================================================================*/
static char *epub2txt_unescape_html (const char *s)
  {
  typedef enum {MODE_ANY=0, MODE_AMP=1} Mode;
  Mode mode = MODE_ANY;
  String *out = string_create_empty();
  WString *in = wstring_create_from_utf8(s);
  WString *ent = wstring_create_empty();
  int i, l = wstring_length (in);
  for (i = 0; i < l; i++)
    {
    const uint32_t *ws = wstring_wstr (in);
    uint32_t c = ws[i]; // Changed from int to uint32_t
    if (mode == MODE_AMP)
      {
      if (c == ';')
        {
        WString *trans = xhtml_translate_entity (ent);
        char *trans_utf8 = wstring_to_utf8 (trans);
        string_append (out, trans_utf8);
        free (trans_utf8);
        wstring_destroy (trans);
        wstring_clear (ent);
        mode = MODE_ANY;
        }
      else
        {
        wstring_append_c (ent, c);
        }
      }
    else
      {
      if (c == '&')
        mode = MODE_AMP;
      else
        // Assuming string_append_c in custom_string.h correctly handles uint32_t for UTF-8
        string_append_c (out, c);
      }
    }
  wstring_destroy (ent);
  wstring_destroy (in);
  char *ret = strdup (string_cstr (out));
  string_destroy (out);
  return ret;
  }

/*============================================================================
  epub2txt_format_meta
============================================================================*/
static void epub2txt_format_meta (const Epub2TxtOptions *options,
          const char *key, const char *text)
  {
  if (text)
    {
    char *ss = epub2txt_unescape_html (text);
    char *s = NULL; // Initialize to NULL
    asprintf (&s, "%s: %s", key, ss);
    char *error = NULL;
    xhtml_utf8_to_stdout (s, options, &error);
    if (error) free (error);
    if (s) free (s); // Check if s was allocated
    free (ss);
    }
  }

/*============================================================================
  epub2txt_dump_metadata
============================================================================*/
static List *epub2txt_dump_metadata (const char *opf_canonical_path,
        const Epub2TxtOptions *options, char **error)
  {
  IN
  List *ret = NULL;
  String *buff = NULL;
  if (string_create_from_utf8_file (opf_canonical_path, &buff, error))
    {
    const char *buff_cstr = string_cstr (buff);
    log_debug ("Read OPF, size %d from %s", string_length (buff), opf_canonical_path);
    XMLNode *metadata_node = NULL; // Renamed variable
    XMLDoc doc;
    XMLDoc_init (&doc);
    if (XMLDoc_parse_buffer_DOM (buff_cstr, APPNAME, &doc))
      {
      XMLNode *root = XMLDoc_root (&doc);
      if (root && root->children)
        {
        int i, l = root->n_children;
        for (i = 0; i < l; i++)
          {
          XMLNode *r1 = root->children[i];
          if (strcmp (r1->tag, "metadata") == 0 || strstr (r1->tag, ":metadata"))
            {
            metadata_node = r1;
            if (metadata_node && metadata_node->children)
            {
            int j, l2 = metadata_node->n_children;
            for (j = 0; j < l2; j++)
              {
              XMLNode *r2 = metadata_node->children[j];
              const char *mdtag = r2->tag;
              const char *mdtext = r2->text;
              if (!mdtext) continue;
              if (strstr (mdtag, "creator"))
                epub2txt_format_meta (options, "Creator", mdtext);
              else if (strstr (mdtag, "publisher"))
                epub2txt_format_meta (options, "Publisher", mdtext);
              else if (strstr (mdtag, "contributor"))
                epub2txt_format_meta (options, "Contributor", mdtext);
              else if (strstr (mdtag, "identifier"))
                epub2txt_format_meta (options, "Identifier", mdtext);
              else if (strstr (mdtag, "date"))
                {
                char *mdate = strdup (mdtext);
                char *p = strchr (mdate, '-');
                if (p) *p = 0;
                epub2txt_format_meta (options, "Date", mdate);
                free (mdate);
                }
              else if (strstr (mdtag, "description"))
                epub2txt_format_meta (options, "Description", mdtext);
              else if (strstr (mdtag, "subject"))
                epub2txt_format_meta (options, "Subject", mdtext);
              else if (strstr (mdtag, "language"))
                epub2txt_format_meta (options, "Language", mdtext);
              else if (strstr (mdtag, "title"))
                epub2txt_format_meta (options, "Title", mdtext);
              else if (strstr (mdtag, "meta") && options->calibre)
                {
                // More robust Calibre metadata parsing
                char *meta_name_attr = NULL;
                char *meta_content_attr = NULL;
                int k, nattrs = r2->n_attributes;

                for (k = 0; k < nattrs; k++) {
                    if (strcmp(r2->attributes[k].name, "name") == 0 || strcmp(r2->attributes[k].name, "property") == 0) {
                        meta_name_attr = r2->attributes[k].value;
                    } else if (strcmp(r2->attributes[k].name, "content") == 0) {
                        meta_content_attr = r2->attributes[k].value;
                    }
                }

                if (meta_name_attr && meta_content_attr) {
                    if (strcmp(meta_name_attr, "calibre:series") == 0) {
                        epub2txt_format_meta(options, "Calibre series", meta_content_attr);
                    } else if (strcmp(meta_name_attr, "calibre:series_index") == 0) {
                        char *s = strdup(meta_content_attr);
                        char *p = strchr(s, '.');
                        if (p) *p = 0;
                        epub2txt_format_meta(options, "Calibre series index", s);
                        free(s);
                    } else if (strcmp(meta_name_attr, "calibre:title_sort") == 0) {
                        epub2txt_format_meta(options, "Calibre title sort", meta_content_attr);
                    }
                }
                }
              }
            }
            break; // Found metadata node
            }
          }
        } else {
            log_warning("Root element or its children are NULL in OPF: %s", opf_canonical_path);
        }
      XMLDoc_free (&doc);
      }
    else
      {
      // Error already contains "Can't parse OPF XML" from XMLDoc_parse_buffer_DOM
      // or asprintf (error, "Can't parse OPF XML from %s", opf_canonical_path);
      }
    string_destroy (buff);
    }
  OUT
  return ret;
  }

/*============================================================================
  epub2txt_get_items
============================================================================*/
List *epub2txt_get_items (const char *opf_canonical_path, char **error)
  {
  IN
  List *ret = NULL;
  String *buff = NULL;
  if (string_create_from_utf8_file (opf_canonical_path, &buff, error))
    {
    const char *buff_cstr = string_cstr (buff);
    log_debug ("Read OPF for spine items, size %d from %s", string_length (buff), opf_canonical_path);
    BOOL got_manifest = FALSE;
    XMLNode *manifest_node = NULL; // Renamed
    XMLDoc doc;
    XMLDoc_init (&doc);
    if (XMLDoc_parse_buffer_DOM (buff_cstr, APPNAME, &doc))
      {
      XMLNode *root = XMLDoc_root (&doc);
      int l_root_children = 0;

      if (root && root->children)
        {
        l_root_children = root->n_children;
        for (int i = 0; i < l_root_children; i++)
          {
          XMLNode *r1 = root->children[i];
          if (strcmp (r1->tag, "manifest") == 0 || strstr (r1->tag, ":manifest"))
            {
            manifest_node = r1;
            got_manifest = TRUE;
            break;
            }
          }
        }
      else
        {
        log_warning ("'%s' has no root element or children -- corrupt EPUB?", opf_canonical_path);
        }

      if (!got_manifest || !manifest_node || !manifest_node->children)
        {
        asprintf (error, "File %s has no valid manifest or manifest children", opf_canonical_path);
        string_destroy(buff);
        XMLDoc_free(&doc);
        OUT
        return NULL;
        }

      ret = list_create_strings();

      if (root && root->children)
      {
      for (int i = 0; i < l_root_children; i++)
        {
        XMLNode *r1 = root->children[i];
        if (strcmp (r1->tag, "spine") == 0 || strstr (r1->tag, ":spine"))
          {
          if (r1->children)
          {
          int j, l_spine_children = r1->n_children;
          for (j = 0; j < l_spine_children; j++)
            {
            XMLNode *itemref_node = r1->children[j]; // itemref
            if (itemref_node->attributes)
            {
            int k, nattrs_itemref = itemref_node->n_attributes;
            for (k = 0; k < nattrs_itemref; k++)
              {
              char *attr_name_itemref = itemref_node->attributes[k].name;
              if (strcmp (attr_name_itemref, "idref") == 0)
                {
                char *idref_value = itemref_node->attributes[k].value;
                int m, l_manifest_children = manifest_node->n_children;
                for (m = 0; m < l_manifest_children; m++)
                  {
                  XMLNode *manifest_item_node = manifest_node->children[m];
                  if (manifest_item_node->attributes)
                  {
                  int n, nattrs_manifest_item = manifest_item_node->n_attributes;
                  for (n = 0; n < nattrs_manifest_item; n++)
                    {
                    char *attr_name_manifest = manifest_item_node->attributes[n].name;
                    char *attr_val_manifest = manifest_item_node->attributes[n].value;
                    if (strcmp (attr_name_manifest, "id") == 0 &&
                        idref_value != NULL && attr_val_manifest != NULL && // Add NULL checks
                        strcmp (attr_val_manifest, idref_value) == 0)
                      {
                      for (int p = 0; p < nattrs_manifest_item; p++)
                        {
                        if (strcmp (manifest_item_node->attributes[p].name, "href") == 0)
                          {
                          char *decoded_href = decode_url (manifest_item_node->attributes[p].value);
                          list_append (ret, decoded_href);
                          break; 
                          }
                        }
                      break; 
                      }
                    }
                  }
                  }
                break; 
                }
              }
            }
            }
          }
          break; 
          }
        }
      }
      XMLDoc_free (&doc);
      }
    else
      {
      // Error from XMLDoc_parse_buffer_DOM
      }
    string_destroy (buff);
    }
  OUT
  return ret;
  }

/*============================================================================
  epub2txt_get_root_file
============================================================================*/
String *epub2txt_get_root_file (const char *container_xml_path, char **error)
  {
  IN
  String *ret = NULL;
  String *buff = NULL;
  if (string_create_from_utf8_file (container_xml_path, &buff, error))
    {
    const char *buff_cstr = string_cstr (buff);
    log_debug ("Read container.xml, size %d from %s", string_length (buff), container_xml_path);
    XMLDoc doc;
    XMLDoc_init (&doc);
    if (XMLDoc_parse_buffer_DOM (buff_cstr, APPNAME, &doc))
      {
      XMLNode *root = XMLDoc_root (&doc);
      if (root && root->children)
        {
        int i, l = root->n_children;
        for (i = 0; i < l; i++)
          {
          XMLNode *r1 = root->children[i];
          if (strcmp (r1->tag, "rootfiles") == 0)
            {
            if (r1->children)
            {
            XMLNode *rootfiles_node = r1;
            int j, l2 = rootfiles_node->n_children;
            for (j = 0; j < l2; j++)
              {
              XMLNode *rootfile_node = rootfiles_node->children[j]; // Renamed
              if (strcmp (rootfile_node->tag, "rootfile") == 0)
                {
                if (rootfile_node->attributes)
                {
                int k, nattrs = rootfile_node->n_attributes;
                for (k = 0; k < nattrs; k++)
                  {
                  char *attr_name = rootfile_node->attributes[k].name;
                  char *attr_value = rootfile_node->attributes[k].value;
                  if (strcmp (attr_name, "full-path") == 0)
                    {
                    ret = string_create (attr_value);
                    break;
                    }
                  }
                }
                if (ret) break;
                }
              }
            }
            if (ret) break;
            }
          }
        } else {
           log_warning("Root element or its children are NULL in %s", container_xml_path);
        }
      if (ret == NULL) { // If still NULL after checking all children
          // Avoid overwriting previous error from string_create_from_utf8_file or XMLDoc_parse_buffer_DOM
          if (*error == NULL) { 
            asprintf (error, "%s does not specify a root file via full-path attribute", container_xml_path);
          }
      }
      XMLDoc_free (&doc);
      }
    else
      {
      // Error from XMLDoc_parse_buffer_DOM, *error should be set
      }
    string_destroy (buff);
    }
  OUT
  return ret;
  }

/*============================================================================
  epub2txt_cleanup
============================================================================*/
void epub2txt_cleanup (void)
  {
  if (tempdir)
    {
    log_debug ("Deleting temporary directory: %s", tempdir);
    run_command ((const char *[]){"rm", "-rf", tempdir, NULL}, FALSE);
    free (tempdir); // tempdir was allocated by strdup from template or asprintf
    tempdir = NULL;
    }
  }

/*============================================================================
  epub2txt_do_file
============================================================================*/
void epub2txt_do_file (const char *file, const Epub2TxtOptions *options,
     char **error)
  {
  IN
  *error = NULL;

  log_debug ("epub2txt_do_file: %s", file);
  if (access (file, R_OK) == 0)
    {
    log_debug ("File access OK");

    char *tempbase;
    if (!(tempbase = getenv("TMPDIR")) && !(tempbase = getenv("TMP")))
      tempbase = "/tmp";
    log_debug ("tempbase is: %s", tempbase);

    if (tempdir != NULL) { // Should be cleaned up by atexit or previous call's end
        log_warning("tempdir was not NULL (%s), implies prior cleanup issue or re-entry.", tempdir);
        // Forcing cleanup here might be too aggressive if an atexit handler is also registered
        // free(tempdir); tempdir = NULL; // Or call epub2txt_cleanup carefully.
    }

    char temp_dir_template[PATH_MAX]; // PATH_MAX from <limits.h>
    // snprintf is safer than sprintf
    snprintf(temp_dir_template, PATH_MAX, "%s/epub2txt.%d.XXXXXX", tempbase, getpid());
    temp_dir_template[PATH_MAX - 1] = '\0'; // Ensure null termination if PATH_MAX is hit

    if (mkdtemp(temp_dir_template) == NULL) { // mkdtemp is from <stdlib.h>
        asprintf(error, "Can't create temporary directory using template %s: %s", temp_dir_template, strerror(errno));
        return; // tempdir (global) is still NULL
    }
    tempdir = strdup(temp_dir_template); // Assign to global tempdir
    if (tempdir == NULL) {
        asprintf(error, "Failed to strdup temporary directory path: %s", strerror(errno));
        // Attempt to remove the created directory if we can't store its path
        rmdir(temp_dir_template); // Best effort, might fail if not empty
        return;
    }
    log_debug ("tempdir created: %s", tempdir);

    log_debug ("Running unzip command");
    int unzip_status = run_command ((const char *[]){"unzip", "-o", "-qq", file, "-d", tempdir, NULL}, TRUE);
     if (unzip_status != 0) {
        asprintf(error, "Unzip command failed for %s with status %d", file, unzip_status);
        epub2txt_cleanup(); // Clean up the created tempdir
        return;
    }

    log_debug ("Unzip finished");
    log_debug ("Fix permissions: %s", tempdir);
    run_command((const char *[]){"chmod", "-R", "u+rwX,go+rX,go-w", tempdir, NULL}, FALSE);
    log_debug ("Permissions fixed");

    char *container_xml_path_str;
    asprintf (&container_xml_path_str, "%s/META-INF/container.xml", tempdir);
    if (!container_xml_path_str) { /* Malloc error */ *error = strdup("asprintf failed for container_xml_path"); epub2txt_cleanup(); return; }
    log_debug ("Container.xml path is: %s", container_xml_path_str);

    String *rootfile_relative_path = epub2txt_get_root_file (container_xml_path_str, error);
    free(container_xml_path_str);

    if (*error == NULL && rootfile_relative_path != NULL)
      {
      log_debug ("OPF rootfile relative path from container.xml: %s", string_cstr(rootfile_relative_path));

      char *opf_constructed_path;
      asprintf (&opf_constructed_path, "%s/%s", tempdir, string_cstr(rootfile_relative_path));
      if (!opf_constructed_path) { /* Malloc error */ /* ... cleanup ... */ string_destroy(rootfile_relative_path); epub2txt_cleanup(); return; }

      char *opf_canonical = realpath (opf_constructed_path, NULL);
      free (opf_constructed_path);

      char *tempdir_canonical_for_check = realpath(tempdir, NULL);
      if (tempdir_canonical_for_check == NULL) {
          asprintf(error, "Failed to resolve temporary directory path '%s': %s", tempdir, strerror(errno));
          string_destroy(rootfile_relative_path);
          if (opf_canonical) free(opf_canonical);
          epub2txt_cleanup();
          return;
      }

      if (opf_canonical == NULL || !is_subpath(tempdir_canonical_for_check, opf_canonical))
        {
        if (opf_canonical == NULL)
          asprintf (error, "Bad OPF rootfile (relative: %s): realpath failed: %s", string_cstr(rootfile_relative_path), strerror (errno));
        else
          asprintf (error, "Bad OPF rootfile path \"%s\": outside EPUB container (resolved temp dir: %s)", opf_canonical, tempdir_canonical_for_check);
        
        free(tempdir_canonical_for_check);
        string_destroy(rootfile_relative_path);
        if (opf_canonical) free(opf_canonical);
        epub2txt_cleanup();
        return;
        }
      free(tempdir_canonical_for_check);

      log_debug("Canonical OPF path: %s", opf_canonical);

      char *content_dir = strdup (opf_canonical);
      if (!content_dir) { /* Malloc error */ 
          asprintf(error, "strdup failed for content_dir"); 
          string_destroy(rootfile_relative_path); 
          free(opf_canonical); 
          epub2txt_cleanup(); 
          return; 
      }
      char *last_slash = strrchr (content_dir, '/');
      if (last_slash) {
          *last_slash = '\0';
      } else {
          // This case means opf_canonical has no '/', which is unlikely for an absolute path
          // unless it's in the root of the filesystem. Default to "." or a copy of tempdir.
          free(content_dir);
          content_dir = strdup(tempdir); // Content is in the root of the temp extraction
          if (!content_dir) { /* Malloc error */ /* ... cleanup ...*/ string_destroy(rootfile_relative_path); free(opf_canonical); epub2txt_cleanup(); return; }
      }
      log_debug ("Content directory is: %s", content_dir);

      if (options->meta)
        {
        epub2txt_dump_metadata (opf_canonical, options, error);
        if (*error)
          {
          log_warning ("Error during metadata dump: %s (continuing with text)", *error);
          free (*error);
          *error = NULL;
          }
        }

      if (!options->notext && *error == NULL)
        {
        List *spine_items = epub2txt_get_items (opf_canonical, error);
        if (*error == NULL && spine_items != NULL)
          {
          log_debug ("EPUB spine has %d items", list_length (spine_items));
          int i, l = list_length (spine_items);
          for (i = 0; i < l; i++)
            {
            const char *item_rel_path = (const char *)list_get (spine_items, i);
            char *item_constr_path;
            asprintf (&item_constr_path, "%s/%s", content_dir, item_rel_path);
            if (!item_constr_path) { /* Malloc error */ continue; } // Skip item

            char *item_canon_path = realpath (item_constr_path, NULL);
            free (item_constr_path);

            if (item_canon_path == NULL || !is_subpath (content_dir, item_canon_path))
              {
              if (item_canon_path == NULL)
                log_warning ("Skipping EPUB spine item \"%s\": invalid path (realpath: %s)",
                  item_rel_path, strerror(errno));
              else
                log_warning ("Skipping EPUB spine item \"%s\" (%s): outside content directory (%s)",
                  item_rel_path, item_canon_path, content_dir);
              if(item_canon_path) free(item_canon_path);
              continue;
              }

            if (options->section_separator)
              printf ("%s\n", options->section_separator);

            xhtml_file_to_stdout (item_canon_path, options, error);
            free(item_canon_path);
            if (*error) {
                log_warning("Error processing spine item %s: %s (continuing)", item_rel_path, *error);
                free(*error);
                *error = NULL;
            }
            }
          list_destroy (spine_items);
          }
         else if (*error) {
             log_warning("Could not get spine items: %s", *error);
             // *error is kept for main to report
        } else { // spine_items is NULL but no error
             log_warning("Spine items list is NULL but no specific error reported by epub2txt_get_items.");
        }
        }
      free (content_dir);
      free (opf_canonical);
      }
    else if (*error) {
        // Error from epub2txt_get_root_file or rootfile_relative_path is NULL
        // *error is already set
    } else { // rootfile_relative_path is NULL, but no *error set by epub2txt_get_root_file
        asprintf(error, "Failed to get OPF root file path from container.xml (it was NULL).");
    }

    if (rootfile_relative_path) string_destroy (rootfile_relative_path);
    epub2txt_cleanup();
    }
  else
    {
    asprintf (error, "File not found or not readable: %s", file);
    }

  OUT
  }