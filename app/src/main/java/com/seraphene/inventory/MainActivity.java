package com.seraphene.inventory;

import android.app.*;
import android.os.*;
import android.content.*;
import android.graphics.Color;
import android.text.InputType;
import android.view.*;
import android.widget.*;
import org.json.*;
import java.io.*;
import java.math.BigDecimal;
import java.util.Locale;
import java.util.concurrent.*;

public final class MainActivity extends Activity {
    static { System.loadLibrary("tindahan"); }
    private static native String nativeCall(String path,String request);
    private final ExecutorService worker=Executors.newSingleThreadExecutor();
    private LinearLayout root,content,tabs;
    private TextView status;
    private int page=0,offset=0;
    private String search="",pendingExport,dbPath;
    private JSONObject totals=new JSONObject();
    private boolean busy=false;
    interface Done {void accept(JSONObject data) throws Exception;}
    interface Action {void run() throws Exception;}
    private int dp(int n){return (int)(n*getResources().getDisplayMetrics().density);}
    private JSONObject obj(Object... pairs) throws Exception {JSONObject o=new JSONObject();for(int i=0;i<pairs.length;i+=2)o.put((String)pairs[i],pairs[i+1]);return o;}
    private String money(long n){return String.format(Locale.US,"₱%,.2f",n/100.0);}
    private String decimal(long n){return BigDecimal.valueOf(n,2).toPlainString();}
    @Override public void onCreate(Bundle state){
        super.onCreate(state);
        if(state!=null){page=state.getInt("page");offset=state.getInt("offset");search=state.getString("search","");pendingExport=state.getString("export");}
        dbPath=getDatabasePath("ledger.db").getAbsolutePath();
        getDatabasePath("ledger.db").getParentFile().mkdirs();
        root=new LinearLayout(this);root.setOrientation(1);root.setPadding(dp(12),dp(8),dp(12),0);
        TextView title=text(root,"Tindahan Ledger",22);title.setTextColor(Color.rgb(20,83,45));
        tabs=new LinearLayout(this);root.addView(tabs);
        String[] names={"Summary","Items","Sales","Money Out","Export"};
        for(int i=0;i<names.length;i++){final int p=i;Button b=new Button(this);b.setText(names[i]);b.setTextSize(11);b.setAllCaps(false);b.setPadding(0,0,0,0);tabs.addView(b,new LinearLayout.LayoutParams(0,dp(56),1));b.setOnClickListener(v->{if(!busy){page=p;offset=0;search="";refresh();}});}
        status=text(root,"",14);
        ScrollView scroll=new ScrollView(this);root.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
        content=column();scroll.addView(content);setContentView(root);refresh();
    }
    @Override protected void onSaveInstanceState(Bundle b){super.onSaveInstanceState(b);b.putInt("page",page);b.putInt("offset",offset);b.putString("search",search);b.putString("export",pendingExport);}
    @Override protected void onDestroy(){worker.shutdown();super.onDestroy();}
    private LinearLayout column(){LinearLayout l=new LinearLayout(this);l.setOrientation(1);l.setPadding(0,dp(4),0,dp(12));return l;}
    private TextView text(LinearLayout l,String value,int size){TextView t=new TextView(this);t.setText(value);t.setTextSize(size);t.setPadding(0,dp(5),0,dp(5));l.addView(t);return t;}
    private Button button(LinearLayout l,String label,Action action){Button b=new Button(this);b.setText(label);b.setAllCaps(false);l.addView(b);b.setOnClickListener(v->{if(!busy)try{action.run();}catch(Exception e){error(e);}});return b;}
    private EditText input(LinearLayout l,String label,String initial,boolean numeric,boolean money){
        text(l,label,14);EditText e=new EditText(this);e.setSingleLine(true);e.setText(initial);
        e.setInputType(numeric?InputType.TYPE_CLASS_NUMBER|(money?InputType.TYPE_NUMBER_FLAG_DECIMAL:0):InputType.TYPE_CLASS_TEXT|InputType.TYPE_TEXT_FLAG_CAP_SENTENCES);
        e.setSelectAllOnFocus(true);l.addView(e);return e;
    }
    private long number(EditText e,boolean money)throws Exception{
        String s=e.getText().toString().trim();
        if(!s.matches(money?"[0-9]+(\\.[0-9]{1,2})?":"[0-9]+"))throw new Exception("Enter a valid "+(money?"peso amount.":"whole quantity."));
        try{long n=new BigDecimal(s).movePointRight(money?2:0).longValueExact();if(n>100000000)throw new Exception("Number is too large.");return n;}catch(ArithmeticException ex){throw new Exception("Number is too large.");}
    }
    private void error(Exception e){new AlertDialog.Builder(this).setTitle("Please check").setMessage(e.getMessage()).setPositiveButton("OK",null).show();}
    private JSONObject execute(JSONObject q)throws Exception{JSONObject r=new JSONObject(nativeCall(dbPath,q.toString()));if(!r.getBoolean("ok"))throw new Exception(r.getString("error"));return r.getJSONObject("data");}
    private void call(JSONObject q,Done done){
        if(busy)return;busy=true;status.setText("Working…");
        worker.execute(()->{try{JSONObject data=execute(q);runOnUiThread(()->{busy=false;if(isDestroyed())return;status.setText("");try{done.accept(data);}catch(Exception e){error(e);}});}catch(Exception e){runOnUiThread(()->{busy=false;if(!isDestroyed()){status.setText("");error(e);}});}});
    }
    private void refresh(){try{call(obj("op","list","offset",offset,"search",search),data->{totals=data.getJSONObject("totals");render(data.getJSONArray("items"));});}catch(Exception e){error(e);}}
    private void render(JSONArray items)throws Exception{
        content.removeAllViews();
        for(int i=0;i<tabs.getChildCount();i++){tabs.getChildAt(i).setSelected(i==page);tabs.getChildAt(i).setBackgroundTintList(android.content.res.ColorStateList.valueOf(i==page?0xffb8dabb:0xffeeeeee));}
        if(page==0){
            text(content,"Available store cash",18);text(content,money(totals.getLong("cash")),30);
            text(content,"Total sales: "+money(totals.getLong("sales")),18);
            text(content,"Outside capital added: "+money(totals.getLong("outside")),18);
            text(content,"Stock purchases: "+money(totals.getLong("purchases")),18);
            text(content,"Money taken out: "+money(totals.getLong("out")),18);
            text(content,"Purchases reduce store cash. Total sales remain the recorded sales.",14);
        }else if(page==1||page==2){
            text(content,page==1?"Items & purchases":"Record a sale",22);
            if(page==1)button(content,"Add / restock item",()->purchase(null));
            EditText term=input(content,"Find item",search,false,false);
            button(content,"Search",()->{search=term.getText().toString().trim();offset=0;refresh();});
            if(items.length()==0)text(content,"No items found.",16);
            for(int i=0;i<Math.min(50,items.length());i++){
                JSONObject item=items.getJSONObject(i);
                LinearLayout row=column();content.addView(row);
                text(row,item.getString("name"),19);
                text(row,item.getLong("qty")+" in stock · "+money(item.getLong("price"))+" each",15);
                if(page==1){button(row,"Restock",()->purchase(item));button(row,"Edit details",()->edit(item));}
                else button(row,"Sell",()->sell(item));
            }
            text(content,"Page "+(offset/50+1),14);
            if(offset>0)button(content,"Previous",()->{offset=Math.max(0,offset-50);refresh();});
            if(items.length()>50)button(content,"Next",()->{offset+=50;refresh();});
        }else if(page==3){
            text(content,"Money taken out",22);text(content,"Available: "+money(totals.getLong("cash")),18);
            EditText amount=input(content,"Amount (₱)","",true,true),reason=input(content,"Reason","",false,false);
            button(content,"Save deduction",()->{JSONObject q=obj("op","out","amount",number(amount,true),"reason",reason.getText().toString());call(q,d->{Toast.makeText(this,"Deduction saved",Toast.LENGTH_SHORT).show();refresh();});});
        }else{
            text(content,"Export records",22);
            button(content,"Inventory CSV",()->export("inventory"));
            button(content,"Transactions CSV",()->export("transactions"));
            text(content,"Data is saved on this phone. Clearing cache keeps records. Uninstalling or clearing app data removes them. Save exports regularly.",15);
        }
    }
    private AlertDialog form(String title,LinearLayout fields){
        ScrollView s=new ScrollView(this);s.setPadding(dp(16),0,dp(16),0);s.addView(fields);
        AlertDialog d=new AlertDialog.Builder(this).setTitle(title).setView(s).setPositiveButton("Save",null).setNegativeButton("Cancel",null).create();
        d.setOnCancelListener(v->{});d.show();return d;
    }
    private void saveForm(AlertDialog d,JSONObject q){
        d.getButton(-1).setEnabled(false);d.getButton(-2).setEnabled(false);d.setCancelable(false);
        if(busy)return;busy=true;status.setText("Saving…");
        worker.execute(()->{try{execute(q);runOnUiThread(()->{busy=false;if(isDestroyed())return;status.setText("");d.dismiss();Toast.makeText(this,"Saved",Toast.LENGTH_SHORT).show();refresh();});}catch(Exception e){runOnUiThread(()->{busy=false;if(isDestroyed())return;status.setText("");d.getButton(-1).setEnabled(true);d.getButton(-2).setEnabled(true);d.setCancelable(true);error(e);});}});
    }
    private void purchase(JSONObject item)throws Exception{
        LinearLayout f=column();
        EditText n=input(f,"Item name",item==null?"":item.getString("name"),false,false);
        if(item!=null)n.setEnabled(false);
        EditText q=input(f,"Quantity purchased","",true,false),amount=input(f,"Total money spent (₱)","",true,true),price=input(f,"Selling price per piece (₱)",item==null?"":decimal(item.getLong("price")),true,true);
        text(f,"Pay using",14);Spinner mode=new Spinner(this);
        mode.setAdapter(new ArrayAdapter<String>(this,android.R.layout.simple_spinner_dropdown_item,new String[]{"Store cash + outside shortage","Entirely outside money","Store cash only"}));f.addView(mode);
        TextView preview=text(f,"Available store cash: "+money(totals.getLong("cash")),15);
        Action update=()->{try{long a=number(amount,true),cash=Math.max(0,totals.getLong("cash"));int m=mode.getSelectedItemPosition();long out=m==1?a:m==0?Math.max(0,a-cash):0;preview.setText("Store cash: "+money(a-out)+"\nOutside capital added: "+money(out)+(m==2&&a>cash?"\nInsufficient store cash.":""));}catch(Exception ignored){preview.setText("Enter the total purchase cost.");}};
        amount.addTextChangedListener(new android.text.TextWatcher(){public void beforeTextChanged(CharSequence s,int start,int count,int after){}public void onTextChanged(CharSequence s,int start,int before,int count){try{update.run();}catch(Exception ignored){}}public void afterTextChanged(android.text.Editable e){}});
        mode.setOnItemSelectedListener(new android.widget.AdapterView.OnItemSelectedListener(){public void onNothingSelected(AdapterView<?> p){}public void onItemSelected(AdapterView<?> p,View v,int pos,long id){try{update.run();}catch(Exception ignored){}}});
        AlertDialog d=form("Add / restock item",f);
        d.getButton(-1).setOnClickListener(v->{if(busy)return;try{saveForm(d,obj("op","purchase","name",n.getText().toString(),"qty",number(q,false),"amount",number(amount,true),"price",number(price,true),"mode",new String[]{"split","outside","cash"}[mode.getSelectedItemPosition()]));}catch(Exception e){error(e);}});
    }
    private void sell(JSONObject item)throws Exception{
        LinearLayout f=column();text(f,item.getString("name")+" · "+money(item.getLong("price"))+" each",18);
        EditText q=input(f,"Quantity sold","1",true,false);AlertDialog d=form("Record sale",f);
        d.getButton(-1).setOnClickListener(v->{if(busy)return;try{saveForm(d,obj("op","sale","id",item.getString("id"),"qty",number(q,false)));}catch(Exception e){error(e);}});
    }
    private void edit(JSONObject item)throws Exception{
        LinearLayout f=column();text(f,"Corrections only. Use Restock to record a purchase. Past transactions stay unchanged.",14);
        EditText n=input(f,"Name",item.getString("name"),false,false),q=input(f,"Correct stock quantity",item.getString("qty"),true,false),cost=input(f,"Cost per piece (₱)",decimal(item.getLong("cost")),true,true),price=input(f,"Selling price per piece (₱)",decimal(item.getLong("price")),true,true);
        AlertDialog d=form("Edit item",f);
        d.getButton(-1).setOnClickListener(v->{if(busy)return;try{saveForm(d,obj("op","edit","id",item.getString("id"),"name",n.getText().toString(),"qty",number(q,false),"cost",number(cost,true),"price",number(price,true)));}catch(Exception e){error(e);}});
    }
    private void export(String type){
        if(pendingExport!=null)return;pendingExport=type;
        Intent i=new Intent(Intent.ACTION_CREATE_DOCUMENT);i.addCategory(Intent.CATEGORY_OPENABLE);i.setType("text/csv");i.putExtra(Intent.EXTRA_TITLE,type+".csv");startActivityForResult(i,1);
    }
    @Override protected void onActivityResult(int request,int result,Intent data){
        super.onActivityResult(request,result,data);if(request!=1)return;
        String type=pendingExport;pendingExport=null;
        if(result!=RESULT_OK||data==null||type==null)return;
        busy=true;status.setText("Exporting…");
        android.net.Uri uri=data.getData();
        worker.execute(()->{File temp=null;try{
            temp=File.createTempFile("export-",".csv",getCacheDir());
            execute(obj("op","export","type",type,"path",temp.getAbsolutePath()));
            try(InputStream in=new FileInputStream(temp);OutputStream out=getContentResolver().openOutputStream(uri,"wt")){
                if(out==null)throw new IOException("Could not open output file.");
                byte[] buf=new byte[8192];int n;while((n=in.read(buf))!=-1)out.write(buf,0,n);
            }
            runOnUiThread(()->{busy=false;if(!isDestroyed())status.setText("Export saved.");});
        }catch(Exception e){runOnUiThread(()->{busy=false;if(!isDestroyed()){status.setText("");error(e);}});}finally{if(temp!=null)temp.delete();}});
    }
}
