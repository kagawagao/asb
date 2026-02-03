package com.example.skintest;

import android.Manifest;
import android.app.Activity;
import android.content.pm.PackageManager;
import android.os.Build;
import android.os.Bundle;
import android.os.Environment;
import android.view.View;
import android.widget.Button;
import android.widget.TextView;
import android.widget.Toast;

import java.io.File;

/**
 * 主活动 - 演示如何使用 SkinLoader 加载皮肤包
 */
public class MainActivity extends Activity {
    
    private static final String TAG = "MainActivity";
    private static final int REQUEST_PERMISSION = 100;
    
    // UI 组件
    private TextView titleText;
    private TextView skinPathText;
    private TextView statusText;
    private TextView resourceIdText;
    private TextView logText;
    private View colorPreview;
    private Button loadSkinButton;
    private Button resetSkinButton;
    
    // 皮肤加载器
    private SkinLoader skinLoader;
    
    // 皮肤包配置
    private static final String SKIN_PACKAGE_NAME = "com.example.skin";
    private String skinPath;
    
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);
        
        // 初始化 UI
        initViews();
        
        // 初始化皮肤加载器
        skinLoader = new SkinLoader();
        
        // 设置皮肤包路径
        skinPath = new File(Environment.getExternalStoragePublicDirectory(
                Environment.DIRECTORY_DOWNLOADS), "com.example.skin.skin").getAbsolutePath();
        skinPathText.setText(skinPath);
        
        // 设置按钮监听
        loadSkinButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                loadSkin();
            }
        });
        
        resetSkinButton.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                resetSkin();
            }
        });
        
        // 请求权限
        checkPermissions();
        
        addLog("App started");
        addLog("Skin package name: " + SKIN_PACKAGE_NAME);
        addLog("Skin path: " + skinPath);
    }
    
    private void initViews() {
        titleText = findViewById(R.id.titleText);
        skinPathText = findViewById(R.id.skinPathText);
        statusText = findViewById(R.id.statusText);
        resourceIdText = findViewById(R.id.resourceIdText);
        logText = findViewById(R.id.logText);
        colorPreview = findViewById(R.id.colorPreview);
        loadSkinButton = findViewById(R.id.loadSkinButton);
        resetSkinButton = findViewById(R.id.resetSkinButton);
    }
    
    private void checkPermissions() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            if (checkSelfPermission(Manifest.permission.READ_EXTERNAL_STORAGE)
                    != PackageManager.PERMISSION_GRANTED) {
                requestPermissions(new String[]{
                        Manifest.permission.READ_EXTERNAL_STORAGE,
                        Manifest.permission.WRITE_EXTERNAL_STORAGE
                }, REQUEST_PERMISSION);
            }
        }
    }
    
    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        
        if (requestCode == REQUEST_PERMISSION) {
            if (grantResults.length > 0 && grantResults[0] == PackageManager.PERMISSION_GRANTED) {
                addLog("Storage permission granted");
            } else {
                addLog("Storage permission denied");
                Toast.makeText(this, "需要存储权限来加载皮肤包", Toast.LENGTH_LONG).show();
            }
        }
    }
    
    /**
     * 加载皮肤
     */
    private void loadSkin() {
        addLog("=== Starting skin load ===");
        statusText.setText(R.string.status_loading);
        
        // 检查文件是否存在
        File skinFile = new File(skinPath);
        if (!skinFile.exists()) {
            String error = "Skin file not found: " + skinPath;
            addLog("ERROR: " + error);
            statusText.setText(R.string.status_error);
            Toast.makeText(this, error, Toast.LENGTH_LONG).show();
            return;
        }
        
        addLog("Skin file exists: " + skinFile.length() + " bytes");
        
        // 加载皮肤
        boolean success = skinLoader.loadSkin(skinPath, getResources(), SKIN_PACKAGE_NAME);
        
        if (success) {
            addLog("Skin loaded successfully!");
            statusText.setText(R.string.status_success);
            
            // 测试获取资源 ID
            testResourceIds();
            
            // 应用皮肤颜色
            applySkinColors();
            
            Toast.makeText(this, "皮肤加载成功！", Toast.LENGTH_SHORT).show();
        } else {
            addLog("ERROR: Failed to load skin");
            statusText.setText(R.string.status_error);
            Toast.makeText(this, "皮肤加载失败，请查看日志", Toast.LENGTH_LONG).show();
        }
    }
    
    /**
     * 测试获取各种资源 ID
     */
    private void testResourceIds() {
        addLog("--- Testing Resource IDs ---");
        
        String[] colorNames = {
            "primary_color", "primary_dark_color", "accent_color",
            "background_color", "text_primary"
        };
        
        StringBuilder result = new StringBuilder();
        
        for (String colorName : colorNames) {
            int resId = skinLoader.getResourceId(colorName, "color");
            String hexId = resId != 0 ? "0x" + Integer.toHexString(resId) : "NOT FOUND";
            
            addLog(colorName + ": " + hexId);
            result.append(colorName).append(": ").append(hexId).append("\n");
        }
        
        resourceIdText.setText(result.toString());
        
        // 测试字符串资源
        String skinLoadedMsg = skinLoader.getString("skin_loaded", "");
        if (!skinLoadedMsg.isEmpty()) {
            addLog("String resource 'skin_loaded': " + skinLoadedMsg);
        }
    }
    
    /**
     * 应用皮肤颜色
     */
    private void applySkinColors() {
        addLog("--- Applying Skin Colors ---");
        
        // 获取主题色并应用到预览视图
        int primaryColor = skinLoader.getColor("primary_color", 0xFF2196F3);
        colorPreview.setBackgroundColor(primaryColor);
        addLog("Applied primary_color to preview: 0x" + Integer.toHexString(primaryColor));
        
        // 获取文本颜色
        int textColor = skinLoader.getColor("text_primary", 0xFF000000);
        titleText.setTextColor(textColor);
        addLog("Applied text_primary to title: 0x" + Integer.toHexString(textColor));
    }
    
    /**
     * 重置为默认主题
     */
    private void resetSkin() {
        addLog("=== Resetting to default theme ===");
        
        skinLoader.release();
        
        // 恢复默认颜色
        colorPreview.setBackgroundColor(getResources().getColor(R.color.default_primary, null));
        titleText.setTextColor(getResources().getColor(R.color.default_text, null));
        
        statusText.setText(R.string.status_default);
        resourceIdText.setText("Not loaded");
        
        addLog("Skin reset to default");
        Toast.makeText(this, "已重置为默认主题", Toast.LENGTH_SHORT).show();
    }
    
    /**
     * 添加日志到界面
     */
    private void addLog(String message) {
        logText.append(message + "\n");
        
        // 自动滚动到底部
        final TextView logTextView = logText;
        logTextView.post(new Runnable() {
            @Override
            public void run() {
                logTextView.requestLayout();
            }
        });
    }
}
