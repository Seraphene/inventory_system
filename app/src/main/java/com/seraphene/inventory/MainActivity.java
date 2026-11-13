package com.seraphene.inventory;

import android.app.Activity;
import android.os.Bundle;
import android.content.Intent;
import android.content.ContentValues;
import android.database.Cursor;
import android.database.sqlite.SQLiteDatabase;
import android.webkit.*;
import android.widget.Toast;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import org.json.JSONObject;

public final class MainActivity extends Activity {
    private SQLiteDatabase db;
    private String pendingExport;
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        if(state!=null) pendingExport=state.getString("export");
        db=openOrCreateDatabase("ledger.db",MODE_PRIVATE,null);
        db.execSQL("CREATE TABLE IF NOT EXISTS ledger (id INTEGER PRIMARY KEY CHECK(id=1), data TEXT NOT NULL)");
        WebView view=new WebView(this);
        view.setWebViewClient(new WebViewClient(){
            @Override public boolean shouldOverrideUrlLoading(WebView v,WebResourceRequest r){return true;}
        });
        view.setWebChromeClient(new WebChromeClient());
        view.getSettings().setJavaScriptEnabled(true);
        view.getSettings().setAllowFileAccess(false);
        view.getSettings().setAllowContentAccess(false);
        view.addJavascriptInterface(new Storage(),"Ledger");
        setContentView(view);
        view.loadUrl("file:///android_asset/index.html");
    }
    public final class Storage {
        @JavascriptInterface public synchronized String load(){
            try(Cursor c=db.rawQuery("SELECT data FROM ledger WHERE id=1",null)){
                return c.moveToFirst()?c.getString(0):"{\"items\":[],\"sales\":[],\"purchases\":[],\"adjustments\":[]}";
            }
        }
        @JavascriptInterface public synchronized boolean save(String data){
            try{
                new JSONObject(data);
                ContentValues v=new ContentValues();v.put("id",1);v.put("data",data);
                return db.insertWithOnConflict("ledger",null,v,SQLiteDatabase.CONFLICT_REPLACE)!=-1;
            }catch(Exception e){return false;}
        }
        @JavascriptInterface public void export(String filename,String data){
            runOnUiThread(()->{
                if(pendingExport!=null)return;
                pendingExport=data;
                Intent intent=new Intent(Intent.ACTION_CREATE_DOCUMENT);
                intent.addCategory(Intent.CATEGORY_OPENABLE);
                intent.setType(filename.endsWith(".json")?"application/json":"text/csv");
                intent.putExtra(Intent.EXTRA_TITLE,filename);
                startActivityForResult(intent,1);
            });
        }
    }
    @Override protected void onSaveInstanceState(Bundle b){super.onSaveInstanceState(b);b.putString("export",pendingExport);}
    @Override protected void onActivityResult(int request,int result,Intent data){
        super.onActivityResult(request,result,data);
        if(request!=1)return;
        if(result==RESULT_OK && data!=null && pendingExport!=null){
            try(OutputStream out=getContentResolver().openOutputStream(data.getData())){
                out.write(pendingExport.getBytes(StandardCharsets.UTF_8));
                Toast.makeText(this,"Export saved",Toast.LENGTH_SHORT).show();
            }catch(Exception e){Toast.makeText(this,"Export failed. Please try again.",Toast.LENGTH_LONG).show();}
        }
        pendingExport=null;
    }
}
